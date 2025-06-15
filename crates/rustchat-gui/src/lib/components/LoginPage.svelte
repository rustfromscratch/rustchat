<script lang="ts">
  import { onMount } from 'svelte';
  import { authApi, setTokens } from '../api';
  import { actions } from '../store';

  let email = $state('');
  let password = $state('');
  let confirmPassword = $state('');
  let verificationCode = $state('');
  let isRegisterMode = $state(false);
  let showVerification = $state(false);
  let loading = $state(false);
  let error = $state('');
  let success = $state('');
  let serverConnected = $state(false);

  // 在组件挂载时检查服务器连接
  onMount(async () => {
    await checkServerConnection();
  });

  async function checkServerConnection() {
    try {
      const response = await fetch('http://127.0.0.1:8080/health');
      serverConnected = response.ok;
      if (!serverConnected) {
        error = 'Cannot connect to server. Please ensure the RustChat server is running.';
      }
    } catch (e) {
      serverConnected = false;
      error = 'Cannot connect to server. Please ensure the RustChat server is running on port 8080.';
    }
  }

  async function handleLogin() {
    if (!email || !password) {
      error = 'Please fill in all fields';
      return;
    }

    loading = true;
    error = '';

    try {
      const response = await authApi.login(email, password);      if (response.data) {
        // 保存令牌
        const { user, ...tokens } = response.data;
        setTokens(tokens);
        
        // 设置用户信息到store和localStorage
        actions.setUser(user);
        actions.setAuthToken(tokens.access_token);
        localStorage.setItem('user_info', JSON.stringify(user));
        
        success = 'Login successful! Redirecting...';
        
        // 延迟后自动跳转到主界面
        setTimeout(() => {
          window.location.reload(); // 刷新页面以进入主界面
        }, 1000);
      } else if (response.error) {
        error = response.error;
      }
    } catch (err: any) {
      console.error('Login error:', err);
      error = err.response?.data?.message || err.message || 'Login failed';
    } finally {
      loading = false;
    }
  }

  async function handleRegister() {
    if (!email || !password || !confirmPassword) {
      error = 'Please fill in all fields';
      return;
    }

    if (password !== confirmPassword) {
      error = 'Passwords do not match';
      return;
    }

    if (password.length < 6) {
      error = 'Password must be at least 6 characters long';
      return;
    }

    loading = true;
    error = '';

    try {
      const response = await authApi.register(email, password);
      
      if (response.data) {
        showVerification = true;
        success = 'Registration successful! Please check your email for verification code.';
      } else if (response.error) {
        error = response.error;
      }
    } catch (err: any) {
      error = err.response?.data?.error || 'Registration failed';
    } finally {
      loading = false;
    }
  }

  async function handleVerification() {
    if (!verificationCode) {
      error = 'Please enter verification code';
      return;
    }

    loading = true;
    error = '';

    try {
      const response = await authApi.verifyEmail(email, verificationCode);
      
      if (response.data) {
        success = 'Email verified successfully! You can now login.';
        showVerification = false;
        isRegisterMode = false;
        verificationCode = '';
      } else if (response.error) {
        error = response.error;
      }
    } catch (err: any) {
      error = err.response?.data?.error || 'Verification failed';
    } finally {
      loading = false;
    }
  }

  async function handleResendCode() {
    loading = true;
    error = '';

    try {
      const response = await authApi.resendCode(email);
      
      if (response.data) {
        success = 'Verification code sent successfully!';
      } else if (response.error) {
        error = response.error;
      }
    } catch (err: any) {
      error = err.response?.data?.error || 'Failed to resend code';
    } finally {
      loading = false;
    }
  }

  function toggleMode() {
    isRegisterMode = !isRegisterMode;
    showVerification = false;
    error = '';
    success = '';
    verificationCode = '';
  }

  function resetForm() {
    email = '';
    password = '';
    confirmPassword = '';
    verificationCode = '';
    error = '';
    success = '';
    showVerification = false;
  }
</script>

<div class="login-container">
  <div class="login-card">
    <div class="logo">
      <h1>🦀 RustChat</h1>
    </div>

    <!-- Server connection status -->
    <div class="server-status">
      <div class="status-indicator" class:connected={serverConnected} class:disconnected={!serverConnected}></div>
      <span class="status-text">
        {serverConnected ? 'Server Connected' : 'Server Disconnected'}
      </span>
      {#if !serverConnected}
        <button class="retry-btn" onclick={checkServerConnection} disabled={loading}>
          🔄 Retry
        </button>
      {/if}
    </div>

    {#if showVerification}
      <div class="verification-form">
        <h2>Email Verification</h2>
        <p>Please enter the verification code sent to your email:</p>
        
        <div class="form-group">
          <input
            type="text"
            bind:value={verificationCode}
            placeholder="Enter 6-digit code"
            maxlength="6"
            disabled={loading}
          />
        </div>

        {#if error}
          <div class="error">{error}</div>
        {/if}

        {#if success}
          <div class="success">{success}</div>
        {/if}

        <div class="form-actions">
          <button onclick={handleVerification} disabled={loading || !verificationCode}>
            {loading ? 'Verifying...' : 'Verify Email'}
          </button>
          
          <button type="button" onclick={handleResendCode} disabled={loading} class="secondary">
            Resend Code
          </button>
        </div>

        <div class="form-links">
          <button type="button" onclick={() => { showVerification = false; }} class="link">
            Back to Registration
          </button>
        </div>
      </div>
    {:else}
      <div class="auth-form">
        <h2>{isRegisterMode ? 'Sign Up' : 'Sign In'}</h2>
        
        <div class="form-group">
          <label for="email">Email</label>
          <input
            id="email"
            type="email"
            bind:value={email}
            placeholder="Enter your email"
            disabled={loading}
            autocomplete="email"
          />
        </div>

        <div class="form-group">
          <label for="password">Password</label>
          <input
            id="password"
            type="password"
            bind:value={password}
            placeholder="Enter your password"
            disabled={loading}
            autocomplete={isRegisterMode ? 'new-password' : 'current-password'}
          />
        </div>

        {#if isRegisterMode}
          <div class="form-group">
            <label for="confirmPassword">Confirm Password</label>
            <input
              id="confirmPassword"
              type="password"
              bind:value={confirmPassword}
              placeholder="Confirm your password"
              disabled={loading}
              autocomplete="new-password"
            />
          </div>
        {/if}

        {#if error}
          <div class="error">{error}</div>
        {/if}

        {#if success}
          <div class="success">{success}</div>
        {/if}

        <div class="form-actions">
          <button
            onclick={isRegisterMode ? handleRegister : handleLogin}
            disabled={loading || !email || !password || (isRegisterMode && !confirmPassword)}
          >
            {loading ? (isRegisterMode ? 'Creating Account...' : 'Signing In...') : (isRegisterMode ? 'Sign Up' : 'Sign In')}
          </button>
        </div>

        <div class="form-links">
          <button type="button" onclick={toggleMode} class="link">
            {isRegisterMode ? 'Already have an account? Sign In' : "Don't have an account? Sign Up"}
          </button>
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  .login-container {
    display: flex;
    align-items: center;
    justify-content: center;
    min-height: 100vh;
    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
    padding: 20px;
  }

  .login-card {
    background: white;
    border-radius: 12px;
    box-shadow: 0 10px 25px rgba(0, 0, 0, 0.1);
    padding: 40px;
    width: 100%;
    max-width: 400px;
  }

  .logo {
    text-align: center;
    margin-bottom: 32px;
  }

  .logo h1 {
    margin: 0;
    color: #333;
    font-size: 28px;
    font-weight: 600;
  }

  .server-status {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    margin-bottom: 20px;
    padding: 8px 12px;
    border-radius: 6px;
    background: #f8f9fa;
    border: 1px solid #e9ecef;
  }

  .status-indicator {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    transition: background-color 0.3s;
  }

  .status-indicator.connected {
    background-color: #28a745;
  }

  .status-indicator.disconnected {
    background-color: #dc3545;
  }

  .status-text {
    font-size: 12px;
    font-weight: 500;
    color: #495057;
  }

  .retry-btn {
    background: none;
    border: none;
    color: #007bff;
    cursor: pointer;
    font-size: 12px;
    padding: 2px 4px;
    border-radius: 3px;
    transition: background-color 0.2s;
  }

  .retry-btn:hover:not(:disabled) {
    background-color: #e3f2fd;
  }

  .retry-btn:disabled {
    opacity: 0.6;
    cursor: not-allowed;
  }

  .auth-form h2,
  .verification-form h2 {
    text-align: center;
    margin-bottom: 24px;
    color: #333;
    font-size: 24px;
    font-weight: 500;
  }

  .verification-form p {
    text-align: center;
    color: #666;
    margin-bottom: 24px;
    line-height: 1.5;
  }

  .form-group {
    margin-bottom: 20px;
  }

  .form-group label {
    display: block;
    margin-bottom: 6px;
    color: #555;
    font-weight: 500;
    font-size: 14px;
  }

  .form-group input {
    width: 100%;
    padding: 12px 16px;
    border: 2px solid #e1e5e9;
    border-radius: 8px;
    font-size: 16px;
    transition: border-color 0.2s ease;
    background: white;
  }

  .form-group input:focus {
    outline: none;
    border-color: #667eea;
  }

  .form-group input:disabled {
    background-color: #f5f5f5;
    cursor: not-allowed;
  }

  .form-actions {
    margin: 24px 0 16px;
  }

  .form-actions button {
    width: 100%;
    padding: 12px 24px;
    border: none;
    border-radius: 8px;
    font-size: 16px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s ease;
    margin-bottom: 12px;
  }

  .form-actions button:not(.secondary) {
    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
    color: white;
  }

  .form-actions button:not(.secondary):hover:not(:disabled) {
    transform: translateY(-2px);
    box-shadow: 0 4px 12px rgba(102, 126, 234, 0.3);
  }

  .form-actions button.secondary {
    background: transparent;
    color: #667eea;
    border: 2px solid #667eea;
  }

  .form-actions button.secondary:hover:not(:disabled) {
    background: #667eea;
    color: white;
  }

  .form-actions button:disabled {
    opacity: 0.6;
    cursor: not-allowed;
    transform: none;
    box-shadow: none;
  }

  .form-links {
    text-align: center;
  }

  .form-links button.link {
    background: none;
    border: none;
    color: #667eea;
    cursor: pointer;
    font-size: 14px;
    text-decoration: underline;
    padding: 0;
  }

  .form-links button.link:hover {
    color: #764ba2;
  }

  .error {
    background: #fee;
    color: #c33;
    padding: 12px 16px;
    border-radius: 6px;
    margin-bottom: 16px;
    font-size: 14px;
    border-left: 4px solid #c33;
  }

  .success {
    background: #efe;
    color: #393;
    padding: 12px 16px;
    border-radius: 6px;
    margin-bottom: 16px;
    font-size: 14px;
    border-left: 4px solid #393;
  }
</style>
