# Prompt cho GitHub AI Agent – Codebase Optimization Staff_Kit (ST)
**Project:** Staff_Kit (ST) Desktop Application  
**Version:** 1.0  
**Align with:** Business Rules Master Spec v1.0 (nguồn sự thật duy nhất) + ST Technical Spec Outline + tất cả file trong thư mục docs/planning/  
**Mục tiêu:** Yêu cầu Agent đọc lại TOÀN BỘ codebase và thực hiện refactor chuyên nghiệp.

## PROMPT CHÍNH THỨC (copy-paste trực tiếp vào GitHub Agent)

Bạn là Senior Full-Stack Architect chuyên Tauri + Rust + React + TypeScript.  
Bạn đang làm việc trên project Staff_Kit (ST) – ứng dụng desktop quản lý tài sản IT cho công ty ASW Việt Nam.

**Yêu cầu bắt buộc:**
1. Đọc lại TOÀN BỘ codebase hiện tại của repository https://github.com/quockhanh2376/Staff_Kit.git (bao gồm src/, src-tauri/, docs/planning/, daily_log.md, tất cả file MD trong docs/planning/).
2. Tuân thủ nghiêm ngặt **Business Rules Master Spec v1.0** (đặc biệt 4 Core Rules, Section 8 Asset Master Data, Section 10 QR, Section 11 Receive, Section 12 Return, Section 16 Search/Filter, Section 19 Audit Log).
3. Tuân thủ **ST Technical Spec Outline** (Tauri/Rust/React architecture, SQLite local-first, Generic MasterData Framework, QR flow…).

**Nhiệm vụ cụ thể bạn phải thực hiện:**
- Tối ưu toàn bộ code: performance, readability, maintainability, scalability.
- Loại bỏ hoàn toàn code thừa, duplicate (đặc biệt sau khi đã triển khai GenericMasterDataList + MasterDataImport).
- Cải thiện error handling: clear, actionable, non-technical message, logging đầy đủ (phân biệt business error vs technical error).
- Đảm bảo typing chặt chẽ (TypeScript strict), Rust safe code, Tauri command boundaries rõ ràng.
- Áp dụng clean architecture: Presentation / Application / Domain / Persistence / Command layers.
- Thêm comment JSDoc / Rust doc khi cần, giữ code self-documenting.
- Tối ưu UI/UX theo ảnh Employee List (search bar + dropdown filter + drag-and-drop columns) và kế hoạch Asset List (search tên máy/mã tài sản/mã nhân viên, dropdown All Categories).
- Align hoàn toàn với QR Receive/Return flow (1 QR chung ban đầu, QR Asset sau này, Request Pending → Approve mới thay đổi official data).
- Đảm bảo mọi action quan trọng đều có Audit Log (Core Rule 4).

**Output mong đợi từ bạn:**
1. Danh sách các file sẽ thay đổi + lý do.
2. Code refactor cho từng file quan trọng (đưa full code mới).
3. File mới nếu cần (ví dụ: thêm helper, utility, error types).
4. Báo cáo tóm tắt: những gì đã tối ưu, code thừa đã xóa, cải thiện maintainability như thế nào.
5. Checklist test cần chạy sau refactor.

Bắt đầu ngay bằng cách xác nhận bạn đã đọc toàn bộ codebase và liệt kê 5 cải tiến lớn nhất bạn sẽ làm. Sau đó thực hiện từng bước một.

Hãy trả lời ngắn gọn, chuyên nghiệp, chỉ đưa code khi thực sự cần thay đổi.