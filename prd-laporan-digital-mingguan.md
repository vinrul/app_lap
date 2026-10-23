# PRD — Aplikasi Laporan Digital Mingguan

## 1. Ringkasan produk
Aplikasi internal untuk mengimpor data mingguan dari Excel, meninjau serta mengelola laporan per periode, dan menghasilkan laporan PDF yang mengikuti format contoh `181026 - LAPORAN MINGGUAN PBM POLDA SULTRA.pdf`. Aplikasi digunakan melalui SPA di browser dan berjalan pada satu komputer dengan backend serta database SQLite lokal.

## 2. Tujuan
- Mengurangi pekerjaan manual untuk menyusun laporan mingguan dari data Excel.
- Menjaga setiap unggahan dan hasil laporan tetap terkait dengan periode minggu yang jelas.
- Memungkinkan operator memeriksa data sebelum menghasilkan PDF.
- Menyediakan riwayat laporan dan kontrol administratif dasar.

## 3. Pengguna dan hak akses
- **Operator**: mengunggah Excel, menetapkan periode, memeriksa hasil impor, menyimpan data periode, melihat/mengelola laporan yang diizinkan, dan menghasilkan PDF.
- **Admin**: seluruh kemampuan operator, ditambah mengelola akun/peran dan pengaturan aplikasi yang diperlukan.
- Autentikasi dan otorisasi diperlukan karena terdapat dua peran. Aplikasi single-machine; mekanisme awal autentikasi lokal dan pengelolaan sesi harus ditetapkan pada desain teknis.

## 4. Ruang lingkup MVP
### Termasuk
1. Login dan pembatasan fitur berdasarkan peran.
2. Dasbor daftar periode/laporan dengan status, tanggal/periode, dan tindakan yang tersedia.
3. Impor workbook `.xlsx` untuk satu periode mingguan.
4. Validasi berkas, pratinjau data, dan ringkasan kesalahan sebelum penyimpanan.
5. Penyimpanan data impor per periode, dengan riwayat unggahan untuk periode tersebut.
6. Peninjauan dan koreksi data yang relevan sebelum laporan dibuat.
7. Pembuatan dan pengunduhan PDF berdasarkan format laporan contoh.
8. Melihat kembali periode terdahulu dan membuat ulang PDF dari data yang tersimpan.
9. Backend Rust menggunakan Axum dan SQLx; SQLite sebagai basis data. Frontend SPA menggunakan Bun, Svelte 5, dan Tailwind CSS versi terbaru yang stabil saat implementasi.

### Tidak termasuk (MVP)
- Akses internet publik, multi-tenant, sinkronisasi cloud, dan aplikasi mobile native.
- Integrasi langsung ke sistem eksternal.
- Pengeditan PDF bebas/desain dokumen umum.
- Penggabungan otomatis data dari beberapa workbook atau sumber selain Excel, kecuali dibutuhkan setelah pemeriksaan format.

## 5. Alur utama
1. Pengguna masuk dan membuka daftar periode.
2. Operator memilih/membuat periode minggu dan mengunggah file Excel.
3. Sistem memeriksa ekstensi/format, sheet dan header yang dikenali, tipe/kelengkapan nilai, serta kemungkinan konflik dengan data periode yang sama.
4. Sistem menampilkan pratinjau dan kesalahan/peringatan yang dapat ditindaklanjuti. Data belum final sebelum operator mengonfirmasi.
5. Operator mengonfirmasi impor; sistem menyimpan unggahan, data terstruktur, waktu, dan identitas pengunggah.
6. Operator meninjau data periode, memperbaiki nilai yang diizinkan, dan melengkapi ringkasan/narasi laporan yang tidak tersedia sebagai kolom khusus di workbook.
7. Sistem menyusun rekap mingguan dari data impor; operator memeriksa angka agregat dan konten editorial sebelum finalisasi.
8. Operator membuat PDF; sistem menyimpan metadata hasil dan menyediakan unduhan.
9. Pengguna membuka riwayat periode dan dapat meninjau data atau membuat ulang PDF.

## 6. Kebutuhan fungsional
- **FR-01 — Akun & peran:** login/logout; endpoint dan antarmuka membatasi aksi sesuai peran admin/operator.
- **FR-02 — Periode mingguan:** pengguna dapat membuat/memilih periode dengan rentang tanggal dan label yang eksplisit; cegah rentang periode ambigu.
- **FR-03 — Impor Excel:** menerima `.xlsx` dengan sheet `Data`; tampilkan nama berkas, 18 kolom yang dikenali, jumlah baris, dan hasil validasi sebelum commit. Header contoh: `Tanggal`, `Waktu kirim (WIB)`, `Satuan`, `Tingkat`, `Polda`, `Polres`, `Polsek`, `Kode`, `Kegiatan`, `Quick Wins`, `Pejabat pelaksana`, `Jabatan`, `Pelaksana`, `Pelapor`, `Status penerimaan`, `Bukti dukung`, `Keluhan personel Polri`, `Isian laporan`.
- **FR-04 — Validasi impor:** petakan header berdasarkan nama, bukan posisi saja; laporkan kesalahan per baris/kolom; bedakan kesalahan yang memblokir impor dari peringatan; jangan menyimpan impor gagal sebagai data final. Validasi rentang tanggal, nilai satuan/wilayah, kode Quick Wins, status penerimaan, dan isi baris; perlakukan narasi `Isian laporan` sebagai konten teks terstruktur/tidak terstruktur yang harus dipertahankan utuh.
- **FR-05 — Konflik/versi unggahan:** setiap unggahan terkait periode. Pengguna melihat unggahan sebelumnya dan mengonfirmasi tindakan atas periode yang sudah memiliki data; jangan menimpa data tanpa konfirmasi dan jejak riwayat.
- **FR-06 — Peninjauan data:** tampilkan data tersimpan per periode dan dukung koreksi field yang disepakati; perubahan tercatat beserta waktu/pengguna.
- **FR-07 — Laporan PDF:** hasil PDF meniru struktur 9 halaman contoh yang diekstrak: (1) sampul/ringkasan angka mingguan; (2) Program Prioritas 4 — Berantas Narkoba; (3) Quick Wins — Green Policing; (4) Quick Wins — Jumat Keliling; (5) Quick Wins — Subuh Keliling; (6) Quick Wins — Minggu Kasih; (7) aspirasi warga dari Subuh Keliling dan Minggu Kasih serta rencana tindak lanjut; (8) tindak lanjut aspirasi melalui kegiatan gabungan/KRYD; (9) amplifikasi media per kegiatan dan penutup. Teks hasil ekstraksi menomori halaman konten sampai halaman 8, sedangkan halaman akhir tidak menampilkan nomor; penomoran final perlu dicocokkan dengan PDF visual. PDF dapat diunduh dan dibuat ulang dari snapshot data periode yang dipilih.
- **FR-07a — Ringkasan dan rekap:** hitung agregat dari baris Excel hanya setelah definisi metrik, periode cakupan, filter status, dan aturan duplikasi disepakati. Sajikan sumber angka dan drill-down ke baris data untuk memudahkan pemeriksaan.
- **FR-07b — Konten editorial/lintas sumber:** sediakan form untuk ringkasan kegiatan, aspirasi, tindak lanjut, pilihan highlight, dan narasi lain yang tidak memiliki kolom sumber eksplisit; labeli data masukan manual dan jangan menyimpulkannya secara otomatis dari teks bebas tanpa persetujuan operator.
- **FR-07c — Visual PDF:** reproduksi header/footer, tipografi, warna, ikon, tabel, grafik, logo, orientasi, ukuran halaman dan penomoran setelah tinjauan visual PDF referensi; pastikan teks terpotong/karakter yang rusak pada ekstraksi diverifikasi dari tampilan aslinya.
- **FR-08 — Riwayat:** pencarian/penyaringan periode dan status, serta akses ke data impor dan dokumen yang terkait.
- **FR-09 — Administrasi:** admin mengelola akun operator/admin dan status aktifnya.
- **FR-10 — Status proses:** antarmuka menyampaikan progres/hasil impor dan pembuatan PDF serta pesan kesalahan yang dapat dipahami.

## 7. Kebutuhan nonfungsional
- **Penyimpanan lokal:** SQLite berada pada lokasi data aplikasi yang persisten, bukan di direktori sementara atau direktori build frontend.
- **Integritas:** penyimpanan periode, unggahan, baris data, audit perubahan, dan metadata PDF konsisten melalui transaksi yang sesuai.
- **Keamanan:** hash kata sandi dengan algoritma yang sesuai; validasi semua input di backend; batasi ukuran dan tipe berkas; jangan mempercayai validasi frontend; lindungi sesi dan endpoint.
- **Privasi:** workbook dan PDF disimpan lokal; kebijakan backup/retensi dan lokasi berkas dikonfigurasi/didokumentasikan.
- **Keandalan:** proses impor dan PDF yang gagal tidak boleh meninggalkan data setengah tersimpan; kesalahan dilog tanpa membocorkan kredensial.
- **Kompatibilitas:** target OS/peramban dan batas ukuran workbook perlu dikonfirmasi. Backend menyediakan API untuk SPA.
- **Versi dependensi:** gunakan versi stabil terbaru yang kompatibel pada saat implementasi, termasuk library Rust; verifikasi status versi ketika dipilih dan kunci versi melalui lockfile agar build dapat direproduksi. "Terbaru" tidak berarti memperbarui dependensi diam-diam sesudah rilis.

## 8. Arsitektur tingkat tinggi
- **Frontend:** SPA Svelte 5 dengan Bun sebagai runtime/tooling dan Tailwind CSS terbaru stabil; konsumsi API HTTP backend.
- **Backend:** layanan Rust dengan Axum; SQLx untuk akses SQLite dan migrasi skema; modul terpisah untuk autentikasi/otorisasi, impor/validasi, domain laporan, penyimpanan, dan pembuatan PDF.
- **Data:** SQLite lokal untuk akun, peran, periode, metadata dan versi unggahan, data laporan terstruktur, audit, serta metadata dokumen. Lokasi/retensi file Excel/PDF ditentukan pada desain teknis.
- **Prinsip batas:** frontend menangani interaksi dan pratinjau; aturan validasi final dan otorisasi ditegakkan backend.

## 9. Model data konseptual
- `users`: identitas, hash kata sandi, peran, status aktif, waktu dibuat.
- `report_periods`: rentang tanggal, label/status, pembuat, waktu dibuat/diubah.
- `imports`: periode, nama/lokasi berkas, checksum atau identitas berkas, pengunggah, status, waktu impor, ringkasan validasi.
- `report_records`: baris impor dengan 18 kolom sumber (`tanggal`, `waktu_kirim_wib`, `satuan`, `tingkat`, `polda`, `polres`, `polsek`, `kode`, `kegiatan`, `quick_wins`, `pejabat_pelaksana`, `jabatan`, `pelaksana`, `pelapor`, `status_penerimaan`, `bukti_dukung`, `keluhan_personel_polri`, `isian_laporan`) serta nomor baris sumber dan tautan ke unggahan.
- `report_sections` atau entitas setara: konten editorial dan field manual per periode/bagian PDF, dengan asal data dan riwayat perubahan.
- `audit_events`: aksi penting (impor, koreksi, finalisasi, pembuatan dokumen, pengelolaan akun) dan pelaku/waktu.
- `generated_reports`: periode, versi data dan konten sumber, lokasi/nama PDF, waktu dibuat, pembuat.

Nama tabel/kolom penyimpanan dapat disesuaikan, tetapi pemetaan header sumber di atas harus dipertahankan dan dapat ditelusuri ke baris Excel asli.

## 10. Kriteria penerimaan MVP
- Operator dapat masuk, mengunggah workbook valid dengan sheet `Data` dan 18 header yang dikenali untuk satu periode, melihat pratinjau beserta jumlah baris dan kesalahan, lalu mengonfirmasi penyimpanan.
- Workbook yang tidak sesuai menghasilkan kesalahan terarah dan tidak mengubah data periode.
- Setiap baris sumber dapat ditelusuri ke nomor baris dan unggahan asal; teks panjang pada `Isian laporan` tidak terpotong saat disimpan.
- PDF uji memiliki bagian laporan dan metrik yang disetujui, dan angka agregat dapat ditelusuri ke baris sumber atau konten manual berlabel.
- Unggahan berulang untuk periode yang sama tidak menghapus data/versi lama tanpa pilihan dan konfirmasi yang eksplisit.
- Pengguna dapat melihat data periode tersimpan; aksi admin tidak tersedia bagi operator dan ditolak pula oleh backend.
- PDF berhasil diunduh dan isi/urutan/layout-nya lolos pemeriksaan terhadap PDF referensi berdasarkan daftar elemen yang disetujui.
- Periode lama tetap dapat ditinjau dan PDF-nya dibuat ulang secara konsisten dari versi data yang dipilih.
- Build frontend/backend, migrasi database, dan test utama berjalan pada lingkungan target yang ditetapkan.

## 11. Asumsi dan keputusan yang sudah disepakati
- Aplikasi digunakan pada satu komputer.
- Peran pengguna: operator dan admin.
- PDF contoh menjadi acuan format, Excel menjadi sumber data.
- Impor dikelola per periode; riwayat unggahan tidak hilang dan penimpaan memerlukan tindakan eksplisit.
- Teknologi: Bun + Svelte 5 SPA + Tailwind CSS terbaru; Rust + Axum + SQLx + SQLite; library Rust stabil terbaru yang kompatibel saat implementasi.

## 12. Temuan dari berkas contoh dan hal yang masih perlu dipastikan
### Excel `report-2026-10-19 (1).xlsx`
- Workbook berhasil dibaca sebagai `.xlsx`; berisi satu sheet bernama `Data`.
- Header berada di baris pertama dan terdiri dari 18 kolom: `Tanggal`, `Waktu kirim (WIB)`, `Satuan`, `Tingkat`, `Polda`, `Polres`, `Polsek`, `Kode`, `Kegiatan`, `Quick Wins`, `Pejabat pelaksana`, `Jabatan`, `Pelaksana`, `Pelapor`, `Status penerimaan`, `Bukti dukung`, `Keluhan personel Polri`, `Isian laporan`.
- Rentang sheet adalah `A1:R4601` (sekitar 4.600 baris data setelah header). Ada 9.927 nilai teks unik pada shared strings; banyak kolom memuat teks panjang, termasuk narasi multi-baris pada `Isian laporan` dan referensi bukti dukung.
- Contoh data menunjukkan tanggal/waktu, hierarki Polda/Polres/Polsek, kode kegiatan seperti `QW5.3` dan `QW7.1`, nama kegiatan, pejabat/pelaksana, status seperti `Tepat waktu`, serta narasi jumlah/hasil/inovasi/dokumentasi.

### PDF `181026 - LAPORAN MINGGUAN PBM POLDA SULTRA.pdf`
- PDF berhasil diekstrak dengan `pdftotext -layout`; teksnya terbaca, termasuk judul, periode, label, angka, dan isi tabel.
- Teks memuat sembilan bagian/halaman: sampul ringkasan; Berantas Narkoba; Green Policing; Jumat Keliling; Subuh Keliling; Minggu Kasih; Aspirasi Warga dan Rencana Tindak Lanjut; Tindak Lanjut Aspirasi/Cipkon Gabungan/KRYD; Amplifikasi Media dan penutup. Footer ekstraksi menunjukkan halaman konten sampai Halaman 8, sementara bagian akhir/penutup tampak sebagai halaman tanpa nomor.
- PDF memuat agregat dan narasi (misalnya total jemaah, kegiatan per wilayah, aspirasi masyarakat, operasi gabungan, dan amplifikasi kanal media) yang tidak semuanya tampak tersedia sebagai kolom khusus di Excel. Asal angka/konten tersebut perlu dipetakan ke baris/narasi sumber atau diinput operator sebagai konten editorial.

### Wajib dikonfirmasi sebelum implementasi final
- Validasi jumlah baris data aktual, rentang tanggal, nilai kosong, variasi header, duplikasi laporan, status yang harus disertakan, serta definisi “tepat waktu”.
- Pemetaan setiap metrik dan tabel PDF ke kolom/baris Excel; tentukan metrik yang tidak dapat dihitung otomatis dan perlu input manual.
- Definisi periode mingguan (contoh PDF menunjukkan 16–18 Oktober 2026) dan cara periode dipilih, disimpan, serta direvisi.
- Pemeriksaan visual halaman PDF untuk logo, warna, grafik, ikon, tipografi, orientasi, ukuran kertas, pemenggalan, dan penomoran; ekstraksi teks saja tidak memverifikasi tampilan.
- Kebijakan mengenai tautan bukti dukung, aset/logo, data sensitif, nama pejabat, dan tanda tangan/cap.
- OS target, siapa yang membuat akun admin pertama, lokasi data lokal, backup, dan retensi.

## 13. Verifikasi produk
- Uji unit validasi dan pemetaan Excel menggunakan workbook contoh dan berkas invalid/edge case.
- Uji integrasi API untuk otorisasi peran, transaksi impor, konflik unggahan, penyimpanan lokal, dan pembuatan PDF.
- Uji alur end-to-end: login → impor → pratinjau → simpan → tinjau → unduh PDF.
- Bandingkan PDF hasil dengan contoh untuk kelengkapan isi dan tata letak pada data representatif.
- Uji pemulihan setelah proses gagal dan pastikan data/versi yang telah tersimpan tetap konsisten.
