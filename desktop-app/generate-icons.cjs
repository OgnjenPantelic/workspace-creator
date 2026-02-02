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

    // Generate main icon (512x512) - used by both platforms
    await sharp(svgContent)
      .resize(512, 512)
      .png()
      .toFile(path.join(iconsDir, 'icon.png'));
    console.log('✓ Generated icon.png (512x512)');

    // Generate ICO for Windows (256x256 is standard for .ico)
    const pngBuffer = await sharp(svgContent)
      .resize(256, 256)
      .png()
      .toBuffer();
    
    const pngToIco = require('png-to-ico');
    const icoBuffer = await pngToIco(pngBuffer);
    fs.writeFileSync(path.join(iconsDir, 'icon.ico'), icoBuffer);
    console.log('✓ Generated icon.ico (256x256)');

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
