// State Management
let mocks = [];
let logs = [];
let eventSource = null;

// DOM Elements
const navItems = document.querySelectorAll('.nav-item');
const tabContents = document.querySelectorAll('.tab-content');
const mockList = document.getElementById('mock-list');
const logsList = document.getElementById('logs-list');
const tabTitle = document.getElementById('tab-title');
const mocksActions = document.getElementById('mocks-actions');
const mockModal = document.getElementById('mock-modal');
const mockForm = document.getElementById('mock-form');
const btnCreateMock = document.getElementById('btn-create-mock');
const btnCloseModal = document.getElementById('btn-close-modal');
const btnCancelModal = document.getElementById('btn-cancel-modal');
const btnClearLogs = document.getElementById('btn-clear-logs');
const btnExport = document.getElementById('btn-export');
const btnImportTrigger = document.getElementById('btn-import-trigger');
const importFile = document.getElementById('import-file');
const btnAddHeader = document.getElementById('btn-add-header');
const headersContainer = document.getElementById('headers-container');
const btnAddReqHeader = document.getElementById('btn-add-req-header');
const reqHeadersContainer = document.getElementById('req-headers-container');
const advancedSection = document.getElementById('advanced-options');
const btnToggleAdvanced = document.getElementById('btn-toggle-advanced');
const jitterToggle = document.getElementById('mock-jitter-enabled');
const jitterSettings = document.getElementById('jitter-settings');

// Test Result Modal Elements
const testModal = document.getElementById('test-modal');
const testResultContent = document.getElementById('test-result-content');
const btnCloseTestModal = document.getElementById('btn-close-test-modal');
const btnCloseTestFooter = document.getElementById('btn-close-test-footer');

// Initialization
document.addEventListener('DOMContentLoaded', () => {
    initTabs();
    loadMocks();
    initLogs();
    setupEventListeners();
});

// Tab Navigation
function initTabs() {
    navItems.forEach(item => {
        item.addEventListener('click', () => {
            const tab = item.getAttribute('data-tab');

            // Update nav status
            navItems.forEach(n => n.classList.remove('active'));
            item.classList.add('active');

            // Update content status
            tabContents.forEach(content => {
                content.classList.remove('active');
                if (content.id === `${tab}-section`) {
                    content.classList.add('active');
                }
            });

            // Update header
            tabTitle.textContent = tab.charAt(0).toUpperCase() + tab.slice(1).replace('-', ' ');
            mocksActions.style.display = tab === 'mocks' ? 'block' : 'none';
        });
    });
}

// API Calls
async function loadMocks() {
    try {
        const res = await fetch('/_admin/mocks');
        mocks = await res.json();
        renderMocks();
    } catch (err) {
        console.error('Failed to load mocks:', err);
    }
}

async function saveMock(mock, isNew = false) {
    const url = isNew ? '/_admin/mocks' : `/_admin/mocks/${mock.id}`;
    const method = isNew ? 'POST' : 'PUT';

    try {
        const res = await fetch(url, {
            method,
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(mock)
        });
        if (res.ok) {
            closeModal();
            loadMocks();
        } else {
            alert('Failed to save mock');
        }
    } catch (err) {
        console.error('Error saving mock:', err);
    }
}

async function deleteMock(id) {
    if (!confirm('Are you sure you want to delete this mock?')) return;

    try {
        const res = await fetch(`/_admin/mocks/${id}`, { method: 'DELETE' });
        if (res.ok) loadMocks();
    } catch (err) {
        console.error('Error deleting mock:', err);
    }
}

async function testMock(mock) {
    console.log("Testing mock:", mock);

    // Show loading state immediately
    const method = mock.condition.method;
    const url = mock.condition.path;
    showTestLoading(method, url);

    try {
        const reqBody = mock.condition.body;
        const reqHeaders = mock.condition.headers || {};

        const options = {
            method,
            headers: { ...reqHeaders }
        };

        if (reqBody) {
            options.body = JSON.stringify(reqBody);
            options.headers['Content-Type'] = 'application/json';
        } else if (['POST', 'PUT', 'PATCH'].includes(method) && mock.response.body) {
            options.body = JSON.stringify(mock.response.body);
            options.headers['Content-Type'] = 'application/json';
        }

        const res = await fetch(url, options);
        const status = res.status;
        const text = await res.text();

        openTestResultModal(method, url, status, text, options.body, options.headers);
        loadMocks(); // Refresh to see the log
    } catch (err) {
        openTestResultModal(method, url, "Error", err.message);
    }
}

function showTestLoading(method, url) {
    testResultContent.innerHTML = `
        <div class="loading-container">
            <div class="spinner"></div>
            <div class="loading-text">Testing ${method} ${url}...</div>
        </div>
    `;
    testModal.style.display = 'flex';
}

function openTestResultModal(method, url, status, responseText, requestBody = null, requestHeaders = null) {
    const headersStr = requestHeaders ? Object.entries(requestHeaders).map(([k, v]) => `${k}: ${v}`).join('\n') : '';

    testResultContent.innerHTML = `
        <div class="test-result-item">
            <span class="test-result-label">Endpoint</span>
            <div class="test-result-value">${method} ${url}</div>
        </div>
        ${headersStr ? `
        <div class="test-result-item">
            <span class="test-result-label">Request Headers Sent</span>
            <div class="test-result-value">${headersStr}</div>
        </div>` : ''}
        ${requestBody ? `
        <div class="test-result-item">
            <span class="test-result-label">Request Body Sent</span>
            <div class="test-result-value">${requestBody}</div>
        </div>` : ''}
        <div class="test-result-item">
            <span class="test-result-label">Status Code</span>
            <div class="test-result-value">${status}</div>
        </div>
        <div class="test-result-item">
            <span class="test-result-label">Response Body</span>
            <div class="test-result-value">${responseText || '(empty)'}</div>
        </div>
    `;
    testModal.style.display = 'flex';
}

function closeTestResultModal() {
    testModal.style.display = 'none';
}

// UI Rendering
function renderMocks() {
    mockList.innerHTML = '';

    if (mocks.length === 0) {
        mockList.innerHTML = '<div class="empty-state">No mocks configured. Create one to get started!</div>';
        return;
    }

    mocks.forEach(mock => {
        const card = document.createElement('div');
        card.className = 'card';
        card.innerHTML = `
            <div class="mock-card-header">
                <span class="method-badge method-${mock.condition.method.toLowerCase()}">${mock.condition.method}</span>
                <span class="status-badge">ID: ${mock.id}</span>
            </div>
            <div class="mock-path">${mock.condition.path}</div>
            <div class="mock-card-footer">
                <div class="status-badge">Return ${mock.response.status_code || 200}</div>
                <div class="card-actions">
                    <button class="btn btn-secondary btn-sm test-btn" title="Test endpoint">Test</button>
                    <button class="btn btn-secondary btn-sm clone-btn" title="Clone mock">Clone</button>
                    <button class="btn btn-secondary btn-sm edit-btn" title="Edit mock">Edit</button>
                    <button class="btn btn-danger btn-sm delete-btn" title="Delete mock">Delete</button>
                </div>
            </div>
        `;

        card.querySelector('.test-btn').onclick = (e) => {
            e.stopPropagation();
            testMock(mock);
        };

        card.querySelector('.clone-btn').onclick = (e) => {
            e.stopPropagation();
            openModal(mock, true);
        };

        card.querySelector('.edit-btn').onclick = (e) => {
            e.stopPropagation();
            openModal(mock);
        };

        card.querySelector('.delete-btn').onclick = (e) => {
            e.stopPropagation();
            deleteMock(mock.id);
        };

        card.onclick = () => openModal(mock);

        mockList.appendChild(card);
    });
}

// Log Streaming
function initLogs() {
    if (eventSource) eventSource.close();

    eventSource = new EventSource('/_admin/logs/stream');
    eventSource.onmessage = (event) => {
        const log = JSON.parse(event.data);
        addLogEntry(log);
    };

    eventSource.onerror = () => {
        console.error('SSE connection lost. Reconnecting...');
    };
}

function addLogEntry(log) {
    const entry = document.createElement('div');
    entry.className = 'log-entry';

    const time = new Date(log.timestamp).toLocaleTimeString();
    const statusClass = log.matched ? 'log-matched' : 'log-missed';
    const statusText = log.matched ? 'MATCH' : 'MISS';

    entry.innerHTML = `
        <span class="log-time">[${time}]</span>
        <span class="log-method">${log.method}</span>
        <span class="log-path">${log.path}</span>
        <span class="${statusClass}">${statusText}</span>
        ${log.expectation_id ? `<span class="log-time">(ID: ${log.expectation_id})</span>` : ''}
    `;

    logsList.prepend(entry);

    // Keep only last 100 logs
    if (logsList.children.length > 100) {
        logsList.removeChild(logsList.lastChild);
    }
}

// Modal Handlers
function openModal(mock = null, isClone = false) {
    const isEdit = !!mock && !isClone;
    if (isClone) {
        document.getElementById('modal-title').textContent = 'Clone Mock';
    } else {
        document.getElementById('modal-title').textContent = isEdit ? 'Edit Mock' : 'Create Mock';
    }

    headersContainer.innerHTML = '';
    reqHeadersContainer.innerHTML = '';

    if (mock) {
        document.getElementById('mock-id').value = isClone ? '' : mock.id;
        document.getElementById('mock-method').value = mock.condition.method;
        document.getElementById('mock-path').value = mock.condition.path;
        document.getElementById('mock-req-body').value = mock.condition.body ? JSON.stringify(mock.condition.body, null, 2) : '';
        document.getElementById('mock-status').value = mock.response.status_code || 200;
        document.getElementById('mock-latency').value = mock.response.latency || 0;

        // Jitter
        if (mock.response.jitter) {
            jitterToggle.checked = true;
            jitterSettings.classList.remove('disabled');
            document.getElementById('mock-jitter-prob').value = (mock.response.jitter.probability * 100).toFixed(0);
            document.getElementById('mock-jitter-status').value = mock.response.jitter.status_code;
            document.getElementById('mock-jitter-body').value = mock.response.jitter.body ? JSON.stringify(mock.response.jitter.body, null, 2) : '';
        } else {
            jitterToggle.checked = false;
            jitterSettings.classList.add('disabled');
        }

        document.getElementById('mock-res-body').value = mock.response.body ? JSON.stringify(mock.response.body, null, 2) : '';

        // Populate request headers
        if (mock.condition.headers) {
            Object.entries(mock.condition.headers).forEach(([key, value]) => {
                addHeaderRow(reqHeadersContainer, key, value);
            });
        }

        // Populate response headers
        if (mock.response.headers) {
            Object.entries(mock.response.headers).forEach(([key, value]) => {
                addHeaderRow(headersContainer, key, value);
            });
        }
    } else {
        mockForm.reset();
        document.getElementById('mock-id').value = '';
        document.getElementById('mock-status').value = 200;
        document.getElementById('mock-latency').value = 0;
        jitterToggle.checked = false;
        jitterSettings.classList.add('disabled');
    }

    // Always hide advanced section on open
    advancedSection.classList.remove('show');
    btnToggleAdvanced.querySelector('.toggle-icon').textContent = '▼';

    mockModal.style.display = 'flex';
}

function addHeaderRow(container, key = '', value = '') {
    const row = document.createElement('div');
    row.className = 'header-row';
    row.innerHTML = `
        <input type="text" placeholder="Key" class="header-key" value="${key}">
        <input type="text" placeholder="Value" class="header-value" value="${value}">
        <button type="button" class="btn-remove-header" title="Remove Header">&times;</button>
    `;

    row.querySelector('.btn-remove-header').onclick = () => row.remove();
    container.appendChild(row);
}

function closeModal() {
    mockModal.style.display = 'none';
}

// Event Listeners
function setupEventListeners() {
    btnCreateMock.onclick = () => openModal();
    btnCloseModal.onclick = closeModal;
    btnCancelModal.onclick = closeModal;
    btnAddHeader.onclick = () => addHeaderRow(headersContainer);
    btnAddReqHeader.onclick = () => addHeaderRow(reqHeadersContainer);

    btnToggleAdvanced.onclick = () => {
        const isShown = advancedSection.classList.toggle('show');
        btnToggleAdvanced.querySelector('.toggle-icon').textContent = isShown ? '▲' : '▼';
    };

    jitterToggle.onchange = (e) => {
        if (e.target.checked) {
            jitterSettings.classList.remove('disabled');
        } else {
            jitterSettings.classList.add('disabled');
        }
    };

    btnCloseTestModal.onclick = closeTestResultModal;
    btnCloseTestFooter.onclick = closeTestResultModal;

    window.onclick = (event) => {
        if (event.target === mockModal) closeModal();
        if (event.target === testModal) closeTestResultModal();
    };

    mockForm.onsubmit = (e) => {
        e.preventDefault();

        const idVal = document.getElementById('mock-id').value;
        const latencyVal = document.getElementById('mock-latency').value;
        const reqBodyStr = document.getElementById('mock-req-body').value;
        const resBodyStr = document.getElementById('mock-res-body').value;

        let requestBody = null;
        let responseBody = null;

        if (reqBodyStr.trim()) {
            try {
                requestBody = JSON.parse(reqBodyStr);
            } catch (err) {
                alert('Invalid JSON in request body');
                return;
            }
        }

        if (resBodyStr.trim()) {
            try {
                responseBody = JSON.parse(resBodyStr);
            } catch (err) {
                alert('Invalid JSON in response body');
                return;
            }
        }

        const responseHeaders = {};
        headersContainer.querySelectorAll('.header-row').forEach(row => {
            const key = row.querySelector('.header-key').value.trim();
            const value = row.querySelector('.header-value').value.trim();
            if (key) responseHeaders[key] = value;
        });

        const requestHeaders = {};
        reqHeadersContainer.querySelectorAll('.header-row').forEach(row => {
            const key = row.querySelector('.header-key').value.trim();
            const value = row.querySelector('.header-value').value.trim();
            if (key) requestHeaders[key] = value;
        });

        const jitterEnabled = jitterToggle.checked;
        let jitterConfig = undefined;
        if (jitterEnabled) {
            const jitterBodyStr = document.getElementById('mock-jitter-body').value;
            let jitterBody = null;
            if (jitterBodyStr.trim()) {
                try { jitterBody = JSON.parse(jitterBodyStr); }
                catch (e) { alert('Invalid JSON in jitter error body'); return; }
            }
            jitterConfig = {
                probability: parseFloat(document.getElementById('mock-jitter-prob').value) / 100,
                status_code: parseInt(document.getElementById('mock-jitter-status').value),
                body: jitterBody
            };
        }

        const mock = {
            id: idVal ? parseInt(idVal) : Math.floor(Math.random() * 1000000),
            condition: {
                method: document.getElementById('mock-method').value,
                path: document.getElementById('mock-path').value,
                body: requestBody,
                headers: Object.keys(requestHeaders).length > 0 ? requestHeaders : undefined
            },
            response: {
                status_code: parseInt(document.getElementById('mock-status').value),
                latency: latencyVal ? parseInt(latencyVal) : undefined,
                jitter: jitterConfig,
                headers: Object.keys(responseHeaders).length > 0 ? responseHeaders : undefined,
                body: responseBody
            }
        };

        saveMock(mock, !idVal);
    };

    btnClearLogs.onclick = () => {
        logsList.innerHTML = '';
    };

    // Export
    btnExport.onclick = async () => {
        try {
            const res = await fetch('/_admin/export');
            const data = await res.json();
            const blob = new Blob([JSON.stringify(data, null, 2)], { type: 'application/json' });
            const url = URL.createObjectURL(blob);
            const a = document.createElement('a');
            a.href = url;
            a.download = `mimicrab-mocks-${new Date().toISOString().slice(0, 10)}.json`;
            a.click();
            URL.revokeObjectURL(url);
        } catch (err) {
            console.error('Export failed:', err);
        }
    };

    // Import
    btnImportTrigger.onclick = () => importFile.click();

    importFile.onchange = async (e) => {
        const file = e.target.files[0];
        if (!file) return;

        const reader = new FileReader();
        reader.onload = async (event) => {
            try {
                const data = JSON.parse(event.target.result);
                if (!Array.isArray(data)) {
                    alert('Invalid format: Expected an array of mocks');
                    return;
                }

                const res = await fetch('/_admin/import', {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify(data)
                });

                if (res.ok) {
                    alert('Import successful!');
                    loadMocks();
                } else {
                    alert('Import failed');
                }
            } catch (err) {
                alert('Invalid JSON file');
            }
        };
        reader.readAsText(file);
    };
}
