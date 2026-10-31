import fs from 'node:fs';
import path from 'node:path';
import writeXlsx from 'write-excel-file/node';

const fixturesDir = path.resolve('src-tauri', 'tests', 'fixtures');
if (!fs.existsSync(fixturesDir)) {
  fs.mkdirSync(fixturesDir, { recursive: true });
}

async function createAssignedAssetFixture() {
  const filePath = path.join(fixturesDir, 'test_assigned_asset.xlsx');
  const data = [
    [
      { value: 'Staff ID', type: String },
      { value: 'Họ và Tên', type: String },
      { value: 'Tại CTY', type: String },
      { value: 'Tại NHÀ', type: String },
    ],
    [
      { value: 'ASWVN030', type: String },
      { value: 'TRẦN THỊ B', type: String },
      { value: 'MON540', type: String },
      { value: '', type: String },
    ],
    [
      { value: 'ASWVN027', type: String },
      { value: 'NGUYỄN VĂN A', type: String },
      { value: 'MON667', type: String },
      { value: '', type: String },
    ],
    [
      { value: 'ASWVN1112', type: String },
      { value: 'VŨ TUẤN KHANG', type: String },
      { value: 'MON700', type: String },
      { value: '', type: String },
    ],
    [
      { value: 'ASWVN958', type: String },
      { value: 'NGUYỄN QUỐC TRỌNG NGHĨA', type: String },
      { value: '', type: String },
      { value: '', type: String },
    ],
    [
      { value: 'ASWVN1019', type: String },
      { value: 'NGUYỄN VĂN C', type: String },
      { value: 'MON586', type: String },
      { value: '', type: String },
    ],
  ];

  const writer = await writeXlsx(data, { sheet: 'New' });
  await writer.toFile(filePath);
  console.log(`Created ${filePath}`);
}

async function createAssetMetadataFixture() {
  const filePath = path.join(fixturesDir, 'test_asset_metadata.xlsx');
  const data = [
    [
      { value: 'Asset Tag', type: String },
      { value: 'Asset Name', type: String },
      { value: 'Category', type: String },
      { value: 'Brand', type: String },
      { value: 'Model', type: String },
      { value: 'Serial Number', type: String },
    ],
    [
      { value: 'MON9999', type: String },
      { value: 'Updated Monitor Name', type: String },
      { value: 'Monitor', type: String },
      { value: 'Dell', type: String },
      { value: 'P2419H', type: String },
      { value: 'SN-TEST-001', type: String },
    ],
    [
      { value: 'HPH0001', type: String },
      { value: 'Headphone 1', type: String },
      { value: 'H Phone', type: String },
      { value: 'Sony', type: String },
      { value: 'WH-1000', type: String },
      { value: 'SN-HPH-001', type: String },
    ],
    [
      { value: 'LAP0001', type: String },
      { value: 'Laptop 1', type: String },
      { value: 'Laptop', type: String },
      { value: 'Dell', type: String },
      { value: 'Latitude', type: String },
      { value: 'SN-LAP-001', type: String },
    ],
  ];

  const writer = await writeXlsx(data, { sheet: 'Asset' });
  await writer.toFile(filePath);
  console.log(`Created ${filePath}`);
}

async function main() {
  await createAssignedAssetFixture();
  await createAssetMetadataFixture();
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
