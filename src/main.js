// Tauri API - v2 compatible
// With withGlobalTauri: true, invoke is available at window.__TAURI__.invoke
let invoke;

function getInvoke() {
    if (invoke) return invoke;
    
    // In Tauri v2 with withGlobalTauri: true, the API is at window.__TAURI__.invoke
    if (window.__TAURI__) {
        if (window.__TAURI__.invoke) {
            invoke = window.__TAURI__.invoke;
        } else if (window.__TAURI__.tauri && window.__TAURI__.tauri.invoke) {
            invoke = window.__TAURI__.tauri.invoke;
        } else if (window.__TAURI__.core && window.__TAURI__.core.invoke) {
            invoke = window.__TAURI__.core.invoke;
        }
    }
    
    return invoke;
}

let isConnected = false;
let isMining = false;
let metricsInterval = null;
let logs = [];

// Initialize - wait for Tauri API to be ready
document.addEventListener('DOMContentLoaded', () => {
    // Wait for Tauri to initialize
    const startTime = Date.now();
    const initInterval = setInterval(() => {
        invoke = getInvoke();
        if (invoke) {
            clearInterval(initInterval);
            setupEventListeners();
            setupLogListener();
            updateUI();
            addLog('Application initialized');
            showStatus('Ready', 'success');
        } else if (Date.now() - startTime > 5000) {
            clearInterval(initInterval);
            console.error('Tauri invoke function not available after 5 seconds');
            console.log('Window objects:', {
                __TAURI__: window.__TAURI__,
                __TAURI_INTERNALS__: window.__TAURI_INTERNALS__
            });
            showStatus('Tauri API not loaded. Check console (F12) for details.', 'error');
            addLog('ERROR: Tauri API not available after 5 seconds');
        }
    }, 100);
});

// Setup Tauri event listener for real-time logs
function setupLogListener() {
    if (window.__TAURI__ && window.__TAURI__.event) {
        // Listen for log events from backend
        window.__TAURI__.event.listen('log', (event) => {
            if (event.payload) {
                addLog(event.payload);
            }
        });
    } else {
        console.warn('Tauri event API not available, logs will not update in real-time');
    }
}

function setupEventListeners() {
    // Connection
    const connectBtn = document.getElementById('connect-btn');
    const disconnectBtn = document.getElementById('disconnect-btn');
    if (connectBtn) {
        connectBtn.addEventListener('click', connectNode);
    }
    if (disconnectBtn) {
        disconnectBtn.addEventListener('click', disconnectNode);
    }
    
    // Mining
    const startBtn = document.getElementById('start-mining-btn');
    const stopBtn = document.getElementById('stop-mining-btn');
    if (startBtn) {
        startBtn.addEventListener('click', startMining);
    }
    if (stopBtn) {
        stopBtn.addEventListener('click', stopMining);
    }
    
    // Threads slider
    const threadsSlider = document.getElementById('threads-slider');
    if (threadsSlider) {
        threadsSlider.addEventListener('input', (e) => {
            const valueDisplay = document.getElementById('threads-value');
            if (valueDisplay) {
                valueDisplay.textContent = e.target.value;
            }
        });
    }
    
    // Logs
    const logsBtn = document.getElementById('show-logs-btn');
    if (logsBtn) {
        logsBtn.addEventListener('click', toggleLogs);
    }
}

// Make toggleSection globally accessible for onclick handlers
window.toggleSection = function(sectionName) {
    const content = document.getElementById(`${sectionName}-content`);
    const chevron = document.getElementById(`${sectionName}-chevron`);
    
    if (!content || !chevron) {
        console.error(`Section elements not found for: ${sectionName}`);
        return;
    }
    
    if (content.classList.contains('collapsed')) {
        content.classList.remove('collapsed');
        chevron.textContent = '▼';
    } else {
        content.classList.add('collapsed');
        chevron.textContent = '▶';
    }
};

// Make toggleLogs globally accessible
window.toggleLogs = function() {
    const panel = document.getElementById('logs-panel');
    if (!panel) {
        console.error('Logs panel not found');
        return;
    }
    if (panel.style.display === 'none' || !panel.style.display) {
        panel.style.display = 'flex';
        updateLogsDisplay();
    } else {
        panel.style.display = 'none';
    }
};

function addLog(message) {
    // Backend already includes timestamp, so use message as-is
    // If message doesn't start with '[', it's a manual log without timestamp
    let logEntry = message;
    if (!message.startsWith('[')) {
        logEntry = `[${new Date().toLocaleTimeString()}] ${message}`;
    }
    logs.push(logEntry);
    if (logs.length > 1000) {
        logs.shift();
    }
    updateLogsDisplay();
}

function updateLogsDisplay() {
    const content = document.getElementById('logs-content');
    if (content) {
        content.innerHTML = logs.map(log => `<div class="log-entry">${escapeHtml(log)}</div>`).join('');
        content.scrollTop = content.scrollHeight;
    }
}

function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

async function connectNode() {
    console.log('connectNode called');
    
    // Get invoke function
    invoke = getInvoke();
    if (!invoke) {
        showStatus('Tauri API not available. Please refresh the app.', 'error');
        addLog('Error: Tauri invoke function not available');
        console.error('Tauri API not available. __TAURI__:', window.__TAURI__);
        return;
    }
    
    const addressInput = document.getElementById('node-address');
    if (!addressInput) {
        showStatus('Address input not found', 'error');
        return;
    }
    
    const address = addressInput.value.trim();
    if (!address) {
        showStatus('Please enter a node address', 'error');
        return;
    }
    
    try {
        addLog(`Connecting to node at ${address}...`);
        showStatus('Connecting...', 'info');
        console.log('Calling invoke with:', { command: 'connect_node', address });
        
        const result = await invoke('connect_node', { address });
        console.log('Invoke result:', result);
        isConnected = true;
        showStatus(result, 'success');
        addLog(`Connected: ${result}`);
        updateUI();
    } catch (error) {
        const errorMsg = error.toString();
        console.error('Connection error:', error);
        showStatus(`Connection failed: ${errorMsg}`, 'error');
        addLog(`Connection error: ${errorMsg}`);
        isConnected = false;
        updateUI();
    }
}

async function disconnectNode() {
    invoke = getInvoke();
    if (!invoke) {
        showStatus('Tauri API not available', 'error');
        return;
    }
    
    try {
        await invoke('disconnect_node');
        isConnected = false;
        if (isMining) {
            await stopMining();
        }
        showStatus('Disconnected from node', 'info');
        addLog('Disconnected from node');
        updateUI();
    } catch (error) {
        showStatus(`Disconnect failed: ${error}`, 'error');
        addLog(`Disconnect error: ${error}`);
    }
}

async function startMining() {
    invoke = getInvoke();
    if (!invoke) {
        showStatus('Tauri API not available', 'error');
        return;
    }
    
    if (!isConnected) {
        showStatus('Please connect to a node first', 'error');
        return;
    }
    
    const miningAddressInput = document.getElementById('mining-address');
    if (!miningAddressInput) {
        showStatus('Mining address input not found', 'error');
        return;
    }
    
    const miningAddress = miningAddressInput.value.trim();
    if (!miningAddress) {
        showStatus('Please enter a mining address', 'error');
        return;
    }
    
    const threadsSlider = document.getElementById('threads-slider');
    const throttleInput = document.getElementById('throttle-ms');
    
    const threads = threadsSlider ? parseInt(threadsSlider.value) : 1;
    const throttleStr = throttleInput ? throttleInput.value.trim() : '';
    const throttleMs = throttleStr ? parseInt(throttleStr) : null;
    
    try {
        addLog(`Starting mining with ${threads} thread(s)...`);
        showStatus('Starting mining...', 'info');
        
        const result = await invoke('start_mining', {
            miningAddress,
            threads,
            throttleMs
        });
        isMining = true;
        showStatus(result, 'success');
        addLog(`Mining started: ${result}`);
        updateUI();
        startMetricsPolling();
    } catch (error) {
        const errorMsg = error.toString();
        showStatus(`Failed to start mining: ${errorMsg}`, 'error');
        addLog(`Mining start error: ${errorMsg}`);
        isMining = false;
        updateUI();
    }
}

async function stopMining() {
    invoke = getInvoke();
    if (!invoke) {
        showStatus('Tauri API not available', 'error');
        return;
    }
    
    try {
        const result = await invoke('stop_mining');
        isMining = false;
        showStatus(result, 'info');
        addLog(`Mining stopped: ${result}`);
        stopMetricsPolling();
        updateUI();
    } catch (error) {
        showStatus(`Failed to stop mining: ${error}`, 'error');
        addLog(`Mining stop error: ${error}`);
    }
}

function startMetricsPolling() {
    if (metricsInterval) {
        clearInterval(metricsInterval);
    }
    
    metricsInterval = setInterval(async () => {
        try {
            const currentInvoke = getInvoke();
            if (!currentInvoke) return;
            const metrics = await currentInvoke('get_metrics');
            const hashesEl = document.getElementById('hashes-tried');
            const submittedEl = document.getElementById('blocks-submitted');
            const acceptedEl = document.getElementById('blocks-accepted');
            
            if (hashesEl) hashesEl.textContent = metrics.hashes_tried.toLocaleString();
            if (submittedEl) submittedEl.textContent = metrics.blocks_submitted.toLocaleString();
            if (acceptedEl) acceptedEl.textContent = metrics.blocks_accepted.toLocaleString();
        } catch (error) {
            // Metrics not available yet - this is normal when not mining
        }
    }, 1000);
}

function stopMetricsPolling() {
    if (metricsInterval) {
        clearInterval(metricsInterval);
        metricsInterval = null;
    }
}

function showStatus(message, type = 'info') {
    const statusBox = document.getElementById('status-message');
    if (statusBox) {
        statusBox.textContent = message;
        statusBox.className = `status-message ${type}`;
    }
}

function updateUI() {
    // Update connection status
    const nodeDot = document.getElementById('node-dot');
    const nodeStatus = document.getElementById('node-status');
    if (nodeDot && nodeStatus) {
        if (isConnected) {
            nodeDot.classList.add('active');
            nodeStatus.textContent = 'Node: Connected';
        } else {
            nodeDot.classList.remove('active');
            nodeStatus.textContent = 'Node: Disconnected';
        }
    }
    
    // Update mining status
    const miningDot = document.getElementById('mining-dot');
    const miningStatus = document.getElementById('mining-status');
    if (miningDot && miningStatus) {
        if (isMining) {
            miningDot.classList.add('active');
            miningStatus.textContent = 'Mining: Active';
        } else {
            miningDot.classList.remove('active');
            miningStatus.textContent = 'Mining: Stopped';
        }
    }
    
    // Update buttons
    const connectBtn = document.getElementById('connect-btn');
    const disconnectBtn = document.getElementById('disconnect-btn');
    const startBtn = document.getElementById('start-mining-btn');
    const stopBtn = document.getElementById('stop-mining-btn');
    
    if (connectBtn) {
        connectBtn.style.display = isConnected ? 'none' : 'inline-block';
    }
    if (disconnectBtn) {
        disconnectBtn.style.display = isConnected ? 'inline-block' : 'none';
    }
    if (startBtn) {
        startBtn.disabled = !isConnected || isMining;
    }
    if (stopBtn) {
        stopBtn.style.display = isMining ? 'inline-block' : 'none';
    }
}
