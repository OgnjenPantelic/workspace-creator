const sharp = require('sharp');
const fs = require('fs');
const path = require('path');

async function generateIcons() {
  const svgContent = fs.readFileSync('public/favicon.svg');
  const iconsDir = 'src-tauri/icons';

  if (!fs.existsSync(iconsDir)) {
    fs.mkdirSync(iconsDir, { recursive: true });
  }

  console.log('Generating icons...');

  // Generate PNG icons at various sizes
  await sharp(svgContent).resize(32, 32).png().toFile(path.join(iconsDir, '32x32.png'));
  console.log('Generated 32x32.png');
  
  await sharp(svgContent).resize(128, 128).png().toFile(path.join(iconsDir, '128x128.png'));
  console.log('Generated 128x128.png');
  
  await sharp(svgContent).resize(256, 256).png().toFile(path.join(iconsDir, '128x128@2x.png'));
  console.log('Generated 128x128@2x.png');
  
  await sharp(svgContent).resize(512, 512).png().toFile(path.join(iconsDir, 'icon.png'));
  console.log('Generated icon.png');

  // Generate ICO for Windows
  const pngBuffer = await sharp(svgContent).resize(256, 256).png().toBuffer();
  const pngToIco = require('png-to-ico');
  const icoBuffer = await pngToIco(pngBuffer);
  fs.writeFileSync(path.join(iconsDir, 'icon.ico'), icoBuffer);
  console.log('Generated icon.ico');

  console.log('\nAll icons generated:');
  console.log(fs.readdirSync(iconsDir));
}

generateIcons().catch(console.error);
