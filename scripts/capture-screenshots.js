const { chromium } = require('playwright');
const path = require('path');
const fs = require('fs');

// Frontend dev server is on 5173, API on 9375
const BASE_URL = 'http://localhost:5173';
const SCREENSHOT_DIR = path.join(__dirname, '..', 'docs', 'img');

if (!fs.existsSync(SCREENSHOT_DIR)) {
  fs.mkdirSync(SCREENSHOT_DIR, { recursive: true });
}

const pages = [
  { name: 'dashboard_light', path: '/dashboard', delay: 3000 },
  { name: 'chat', path: '/chat', delay: 2000 },
  { name: 'agents', path: '/agents', delay: 2000 },
  { name: 'devices', path: '/devices', delay: 2000 },
  { name: 'rules', path: '/rules', delay: 2000 },
  { name: 'transforms', path: '/transforms', delay: 2000 },
  { name: 'messages', path: '/messages', delay: 2000 },
  { name: 'extensions', path: '/extensions', delay: 2000 },
  { name: 'settings', path: '/settings', delay: 2000 },
  { name: 'data-push', path: '/data-push', delay: 2000 },
  { name: 'llm-backends', path: '/llm-backends', delay: 2000 },
];

async function login(page) {
  console.log('Navigating to login...');
  await page.goto(`${BASE_URL}/login`, { waitUntil: 'networkidle', timeout: 30000 });
  await page.waitForTimeout(2000);

  // Use specific selectors from the React components
  const usernameInput = await page.$('#username');
  const passwordInput = await page.$('#password');

  if (!usernameInput || !passwordInput) {
    console.log('Login form not found. Trying generic selectors...');
    // Maybe already logged in or different page state
    const currentUrl = page.url();
    console.log('Current URL:', currentUrl);
    if (!currentUrl.includes('/login')) {
      console.log('Already on a non-login page, proceeding...');
      return true;
    }
    return false;
  }

  console.log('Found login form, filling credentials...');
  await usernameInput.click();
  await usernameInput.fill('Admin');
  await page.waitForTimeout(300);

  await passwordInput.click();
  await passwordInput.fill('zxc707cxz');
  await page.waitForTimeout(300);

  const submitBtn = await page.$('button[type="submit"]');
  if (submitBtn) {
    console.log('Clicking submit...');
    await submitBtn.click();
    // Wait for navigation after login
    await page.waitForURL('**/dashboard**', { timeout: 10000 }).catch(() => {
      console.log('URL did not change to dashboard, checking current URL...');
    });
    await page.waitForTimeout(3000);
    console.log('After login URL:', page.url());
    return true;
  }
  return false;
}

async function captureScreenshots() {
  const browser = await chromium.launch({ headless: true });

  // --- Desktop screenshots ---
  const context = await browser.newContext({
    viewport: { width: 1440, height: 900 },
    locale: 'en-US',
  });
  const page = await context.newPage();

  // Capture login page first (before logging in)
  console.log('\n--- Capturing login page ---');
  await page.goto(`${BASE_URL}/login`, { waitUntil: 'networkidle', timeout: 30000 });
  await page.waitForTimeout(2000);
  await page.screenshot({ path: path.join(SCREENSHOT_DIR, 'login.png') });
  console.log('  ✓ login.png saved');

  // Login
  const loggedIn = await login(page);
  if (!loggedIn) {
    console.log('Login failed, aborting...');
    await browser.close();
    return;
  }

  // Capture each page
  console.log('\n--- Capturing desktop pages ---');
  for (const p of pages) {
    try {
      console.log(`Capturing ${p.name}...`);
      await page.goto(`${BASE_URL}${p.path}`, { waitUntil: 'networkidle', timeout: 20000 });
      await page.waitForTimeout(p.delay);

      await page.screenshot({
        path: path.join(SCREENSHOT_DIR, `${p.name}.png`),
      });
      console.log(`  ✓ ${p.name}.png`);
    } catch (e) {
      console.log(`  ✗ ${p.name}: ${e.message}`);
      // Try screenshot anyway
      try {
        await page.screenshot({ path: path.join(SCREENSHOT_DIR, `${p.name}.png`) });
        console.log(`  ~ ${p.name}.png (captured despite error)`);
      } catch (_) {}
    }
  }

  // --- Dark mode dashboard ---
  console.log('\n--- Capturing dark mode ---');
  try {
    // Set dark theme via localStorage and class
    await page.evaluate(() => {
      localStorage.setItem('theme', 'dark');
    });
    await page.goto(`${BASE_URL}/dashboard`, { waitUntil: 'networkidle', timeout: 20000 });
    await page.evaluate(() => {
      document.documentElement.classList.add('dark');
    });
    await page.waitForTimeout(3000);
    await page.screenshot({ path: path.join(SCREENSHOT_DIR, 'dashboard_dark.png') });
    console.log('  ✓ dashboard_dark.png');
  } catch (e) {
    console.log(`  ✗ dark mode: ${e.message}`);
  }

  // --- Mobile screenshot ---
  console.log('\n--- Capturing mobile ---');
  const mobileContext = await browser.newContext({
    viewport: { width: 390, height: 844 },
    locale: 'en-US',
    hasTouch: true,
    isMobile: true,
  });
  const mobilePage = await mobileContext.newPage();

  await login(mobilePage);
  try {
    await mobilePage.goto(`${BASE_URL}/dashboard`, { waitUntil: 'networkidle', timeout: 20000 });
    await mobilePage.waitForTimeout(2000);
    await mobilePage.screenshot({ path: path.join(SCREENSHOT_DIR, 'mobile_web.png') });
    console.log('  ✓ mobile_web.png');
  } catch (e) {
    console.log(`  ✗ mobile: ${e.message}`);
  }

  await mobileContext.close();
  await context.close();
  await browser.close();
  console.log('\nDone!');
}

captureScreenshots().catch(console.error);
