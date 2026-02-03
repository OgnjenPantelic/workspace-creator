// Platform detection
function detectPlatform() {
    const userAgent = window.navigator.userAgent;
    const platform = window.navigator.platform;
    const macosPlatforms = ['Macintosh', 'MacIntel', 'MacPPC', 'Mac68K'];
    const windowsPlatforms = ['Win32', 'Win64', 'Windows', 'WinCE'];
    
    let detectedOS = null;
    
    if (macosPlatforms.indexOf(platform) !== -1) {
        detectedOS = 'macOS';
    } else if (windowsPlatforms.indexOf(platform) !== -1) {
        detectedOS = 'Windows';
    } else if (/Linux/.test(platform)) {
        detectedOS = 'Linux';
    }
    
    return detectedOS;
}

// Update primary download button based on detected platform
function setupPrimaryDownload() {
    const os = detectPlatform();
    const detectedIcon = document.getElementById('detected-icon');
    const detectedPlatform = document.getElementById('detected-platform');
    const downloadText = document.getElementById('download-text');
    const downloadInfo = document.getElementById('download-info');
    const primaryDownloadBtn = document.getElementById('primary-download');
    
    if (os === 'macOS') {
        detectedIcon.textContent = '🍎';
        detectedPlatform.textContent = 'Download for macOS';
        downloadText.textContent = 'Download DMG';
        downloadInfo.textContent = 'macOS 10.15 or later • ~45 MB';
        primaryDownloadBtn.onclick = () => {
            window.location.href = 'downloads/mac/Databricks-Deployer-1.0.0.dmg';
        };
    } else if (os === 'Windows') {
        detectedIcon.textContent = '🪟';
        detectedPlatform.textContent = 'Download for Windows';
        downloadText.textContent = 'Download MSI';
        downloadInfo.textContent = 'Windows 10 or later • ~40 MB';
        primaryDownloadBtn.onclick = () => {
            window.location.href = 'downloads/windows/Databricks-Deployer-1.0.0.msi';
        };
    } else if (os === 'Linux') {
        detectedIcon.textContent = '🐧';
        detectedPlatform.textContent = 'Linux Version Coming Soon';
        downloadText.textContent = 'Coming Soon';
        primaryDownloadBtn.disabled = true;
        downloadInfo.textContent = 'Linux support is in development';
    } else {
        detectedIcon.textContent = '💻';
        detectedPlatform.textContent = 'Choose Your Platform';
        downloadText.textContent = 'Select Below';
        primaryDownloadBtn.disabled = true;
        downloadInfo.textContent = 'Please select your platform from the options below';
    }
}

// Tab switching for installation instructions
function setupTabs() {
    const tabButtons = document.querySelectorAll('.tab-btn');
    const tabContents = document.querySelectorAll('.tab-content');
    
    tabButtons.forEach(button => {
        button.addEventListener('click', () => {
            const targetTab = button.getAttribute('data-tab');
            
            // Remove active class from all tabs and contents
            tabButtons.forEach(btn => btn.classList.remove('active'));
            tabContents.forEach(content => content.classList.remove('active'));
            
            // Add active class to clicked tab and corresponding content
            button.classList.add('active');
            document.getElementById(targetTab).classList.add('active');
        });
    });
}

// Track download clicks (optional - for analytics)
function trackDownload(platform, fileType) {
    console.log(`Download started: ${platform} ${fileType}`);
    
    // You can integrate with analytics here:
    // gtag('event', 'download', { platform: platform, file_type: fileType });
    // or
    // plausible('Download', { props: { platform: platform, type: fileType } });
}

// Add download tracking to all download links
function setupDownloadTracking() {
    const downloadLinks = document.querySelectorAll('a[download], a[href*="downloads/"]');
    
    downloadLinks.forEach(link => {
        link.addEventListener('click', (e) => {
            const href = link.getAttribute('href');
            const platform = href.includes('mac') ? 'macOS' : 
                           href.includes('windows') ? 'Windows' : 'Unknown';
            const fileType = href.includes('.dmg') ? 'DMG' : 
                           href.includes('.msi') ? 'MSI' : 
                           href.includes('.AppImage') ? 'AppImage' : 'Unknown';
            
            trackDownload(platform, fileType);
        });
    });
}

// Initialize everything when DOM is loaded
document.addEventListener('DOMContentLoaded', () => {
    setupPrimaryDownload();
    setupTabs();
    setupDownloadTracking();
});
