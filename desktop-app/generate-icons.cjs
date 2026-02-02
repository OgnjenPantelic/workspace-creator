const sharp = require('sharp');
const fs = require('fs');
const path = require('path');

async function generateIcons() {
  try {
    const svgPath = 'public/favicon.svg';
    const iconsDir = 'src-tauri/icons';

    if (!fs.existsSync(svgPath)) {
      throw new Error(`SVG file not found: ${svgPath}`);
    }

    const svgContent = fs.readFileSync(svgPath);
    console.log('Read SVG file:', svgPath);

    if (!fs.existsSync(iconsDir)) {
      fs.mkdirSync(iconsDir, { recursive: true });
      console.log('Created icons directory:', iconsDir);
    }

    console.log('Generating icons...\n');

    // Generate icons at multiple sizes for best quality on all platforms
    const sizes = [
      { size: 32, name: '32x32.png' },
      { size: 128, name: '128x128.png' },
      { size: 256, name: '128x128@2x.png' },
      { size: 512, name: 'icon.png' },
    ];

    for (const { size, name } of sizes) {
      await sharp(svgContent)
        .resize(size, size)
        .png()
        .toFile(path.join(iconsDir, name));
      console.log(`✓ Generated ${name} (${size}x${size})`);
    }

    // List all generated files
    console.log('\nGenerated files:');
    const files = fs.readdirSync(iconsDir);
    files.forEach(f => {
      const stats = fs.statSync(path.join(iconsDir, f));
      console.log(`  - ${f} (${stats.size} bytes)`);
    });

    console.log('\n✓ Icon generation complete!');
  } catch (error) {
    console.error('Error generating icons:', error);
    process.exit(1);
  }
}

generateIcons();
