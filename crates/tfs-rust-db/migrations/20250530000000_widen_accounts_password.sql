-- Widen accounts.password for bcrypt hashes ($2b$…, ~60 chars).
-- C++ reference: TFS 1.4.2 used CHAR(40) for SHA1 hex; Rust uses VARCHAR(255).

ALTER TABLE `accounts` MODIFY `password` VARCHAR(255) NOT NULL;
