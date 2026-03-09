# Staff Kit - Project Note

Date: 2026-03-04
Owner: IT Team / HR Ops

## 1) Muc tieu du an
- Xay app desktop Staff Kit de quan ly nhan su theo nhom: Employee list, Onboarding, Offboarding, Internal Movement.
- Nguon dau vao chinh la Excel, co the nhieu file va nhieu cau truc cot khac nhau.
- Du lieu phai map theo EE.ID (ma nhan vien) la duy nhat.
- Team IT can quan ly cot hien thi linh hoat theo tung user profile.

## 2) Pham vi nghiep vu da chot
1. Import Excel theo 2 buoc:
- Buoc 1: chon 1 hoac nhieu file Excel.
- Buoc 2: app hien danh sach cot detect duoc de user chon cot can import.

2. Quy tac map du lieu:
- Match theo EE.ID.
- Neu EE.ID da ton tai thi update gia tri cot duoc chon.
- Neu cot dong (dynamic) moi thi tao field dong tu dong.

3. Nhom du lieu (staff group):
- Employee list.
- Onboarding.
- Offboarding.
- Internal Movement (chinh ta da sua).

4. Quan ly cot hien thi:
- User co the an/hien cot.
- User co the reorder cot.
- User co the resize do rong cot.
- User co the rename cot.
- User co the them/xoa cot dynamic.
- Setting cot luu theo user profile dang login.

5. Default view khi login:
- Hien cot mac dinh theo preset da chot (hien tai da bo Nick Name khoi default theo yeu cau moi).
- Moi user dang nhap vao deu apply default truoc, sau do co the luu view rieng.

6. Edit mode:
- Bo cot Actions va bo edit tung dong bang icon but.
- Dung nut Edit/Save o footer.
- Admin double click cell de sua, Save de luu.
- Co hien "da sua bao nhieu dong".

7. Move nhan vien giua cac bang:
- Chon EE.ID trong Edit mode.
- Chon target group.
- Bam Move de chuyen dong.
- Ho tro chon xuyen nhieu trang (across pages).

8. Settings:
- Co Temporary Reset de xoa toan bo data app + DB (dung tam trong giai doan chuan hoa du lieu).
- Import Excel duoc dat trong Settings.

9. Import theo nhom:
- User co the chon import target group truoc khi import.
- Du lieu import duoc do vao dung group da chon.

10. Cot bo qua khi import:
- Bo cot "Question".
- Bo cot canh bao thiet bi dai theo yeu cau.

## 3) Trang thai implementation hien tai (da lam)
1. Backend:
- CRUD employee/team.
- Search/list/pagination.
- Import Excel multi-file + column preview.
- Upsert theo EE.ID.
- Dynamic columns (definition + values).
- API move nhieu employee sang group khac: `move_employees_group`.

2. Frontend:
- Login local account.
- Save column prefs theo user profile.
- Drawer Column Preferences (toggle/reorder/rename/add/delete).
- Bang employee co vertical scroll.
- Row default = 15, co them 30/50/100/500.
- Edit mode + Save edits.
- Move selected rows giua cac group.
- Move selection across pages + Select Page/Unselect Page + Clear Selected.
- Gear settings dat o cot dau tien hien thi va da tang kich thuoc.

3. Database:
- SQLite + FTS trigger da duoc dieu chinh de tranh loi khi wipe data.
- EE.ID unique.
- staff_group da normalize cho cac gia tri legacy.

## 4) Van de da gap va cach xu ly
1. Loi Tauri dialog permission khi Import Excel:
- Da bo sung/can cau hinh permission dung cho dialog open.

2. Loi FTS "cannot DELETE from contentless fts5 table":
- Da sua trigger/reset flow de reset data an toan.

3. Du lieu import update (Computer Name, Sophos, ME) chua vao bang:
- Da chinh luong import de merge theo EE.ID + selected columns.

## 5) Quy tac du lieu can giu trong suot project
1. EE.ID la khoa nghiep vu duy nhat de merge/update.
2. Khong import tat ca cot mot cach vo dieu kien; phai qua buoc chon cot.
3. Moi cot moi tu Excel phai qua normalize key/label truoc khi luu DB.
4. Moi thay doi cot hien thi phai save theo user profile.
5. Cac thao tac move group va edit hang loat chi cho admin.

## 6) Y tuong tiep theo da ban: QR + mobile form
Muc tieu:
- Tao QR de nhan vien scan tren dien thoai.
- Nhan vien dien thong tin nhan/thu hoi thiet bi.
- Du lieu tu dong vao Onboarding hoac Offboarding tuy loai form.

Huong ky thuat de xay dung (de xuat):
1. App tao QR cho 2 loai form: onboarding/offboarding.
2. QR tro den web form mobile (kem token hop le, co han su dung).
3. Mobile submit vao bang tam `mobile_submissions`.
4. App desktop dong bo, validate, sau do upsert vao bang nhan vien va group tuong ung.
5. Co log xu ly: success/failed/reason.

## 7) Rang buoc nhap lieu cho quy trinh nhan laptop (yeu cau IT)
1. Bat buoc field:
- EE.ID.
- Computer Name.
- Laptop Serial.
- So man hinh duoc cap.
- Monitor Serial (neu co man hinh).
- Ngay nhan.

2. Validate:
- Computer Name theo regex chuan IT (se chot mau cu the).
- Serial chi cho phep format hop le.
- Uppercase tu dong.
- Chong trung asset dang active.

3. UX nhap lieu:
- Uu tien dropdown/select tu asset list thay vi go tay.
- Ho tro scan barcode/QR cho serial.
- Khong cho submit neu thieu field bat buoc.

## 8) Backlog uu tien tiep theo (step-by-step)
Step 1:
- Chot form schema cho Onboarding/Offboarding mobile.
- Chot regex Computer Name, rule Serial, rule Monitor.

Step 2:
- Tao bang `mobile_submissions` + API receive submission + API list submissions.

Step 3:
- Tao man hinh QR trong Settings (tao/refresh token, expiry).

Step 4:
- Tao mobile web form + validate client + validate server.

Step 5:
- Tao luong approve/import submissions vao DB chinh, route dung group.

Step 6:
- Them audit log va bo loc trong app desktop.

Step 7:
- Test nghiep vu end-to-end voi file Excel + mobile submissions.

## 9) Cac diem can chot voi stakeholder truoc khi code QR
1. Mobile form host o dau (LAN noi bo hay internet).
2. Co can login user mobile hay chi can token QR.
3. Co can buoc "IT approve" truoc khi ghi vao bang chinh khong.
4. Danh sach asset nguon (co bang asset rieng hay nhap tay).
5. SLA dong bo du lieu (real-time hay theo dot 1-5 phut).

## 10) Nguyen tac lam viec
- Bam dung file note nay de implement tung step.
- Moi step xong cap nhat lai muc "Trang thai implementation" va "Backlog".
- Khong mo rong scope neu chua dong step dang lam.

## 11) Ke hoach refactor de de bao tri

### Thu tu uu tien
1. Tach `src-tauri/src/db.rs` truoc: Rust type system bao ve chat, it rui ro regression hon.
2. Sau do moi tach `src/App.tsx`: can can than hon vi state sharing giua cac components.

### Cau truc muc tieu Backend (db.rs → folder per feature)

```
src-tauri/src/
├── lib.rs              # Chi giu Tauri command wrappers (nhu hien tai)
├── db/
│   ├── mod.rs          # Re-export + init_database + schema constant
│   ├── schema.rs       # SQL schema string duy nhat
│   ├── employee.rs     # Employee CRUD, query, move
│   ├── column.rs       # Column definition + dynamic values
│   ├── team.rs         # Team CRUD
│   ├── auth.rs         # Local accounts + password logic
│   ├── import.rs       # Excel import + preview + column inspect
│   └── backup.rs       # Backup settings + auto backup
└── main.rs
```

### Cau truc muc tieu Frontend (App.tsx → folder per feature)

```
src/
├── components/            # Shared UI primitives (Drawer, Button, ...)
├── features/
│   ├── auth/
│   │   ├── LoginScreen.tsx
│   │   └── useAuth.ts
│   ├── employees/
│   │   ├── EmployeeTable.tsx
│   │   ├── EmployeeEditDrawer.tsx
│   │   ├── useEmployees.ts
│   │   └── useColumnPrefs.ts
│   ├── import/
│   │   └── ImportDrawer.tsx
│   └── settings/
│       └── SettingsDrawer.tsx
└── App.tsx                # Chi con routing + wiring (~100 dong)
```
