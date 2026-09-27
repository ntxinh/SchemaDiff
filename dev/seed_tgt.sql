CREATE DATABASE schemadiff_tgt;
GO
USE schemadiff_tgt;
GO
CREATE TABLE dbo.Users (
    id INT IDENTITY(1,1) NOT NULL,
    name NVARCHAR(100) NOT NULL,
    email NVARCHAR(100) NULL,
    created_at DATETIME2 NULL DEFAULT (SYSDATETIME()),
    CONSTRAINT PK_Users PRIMARY KEY (id)
);
GO
CREATE TABLE dbo.Orders (
    id INT IDENTITY(1,1) NOT NULL,
    user_id INT NOT NULL,
    total DECIMAL(18,2) NOT NULL,
    status NVARCHAR(20) NULL,
    shipped_at DATETIME2 NULL,
    CONSTRAINT PK_Orders PRIMARY KEY (id),
    CONSTRAINT FK_Orders_Users FOREIGN KEY (user_id) REFERENCES dbo.Users(id),
    CONSTRAINT CK_Orders_Total CHECK (total >= 0)
);
GO
CREATE TABLE dbo.Products (
    id INT NOT NULL,
    sku NVARCHAR(20) NOT NULL,
    price DECIMAL(10,2) NOT NULL,
    CONSTRAINT PK_Products PRIMARY KEY (id),
    CONSTRAINT UQ_Products_Sku UNIQUE (sku)
);
GO
CREATE TABLE dbo.AuditLog (id INT NOT NULL, msg NVARCHAR(200) NULL,
    CONSTRAINT PK_AuditLog PRIMARY KEY (id));
GO
CREATE INDEX IX_Users_Name ON dbo.Users(name);
GO
CREATE INDEX IX_Orders_Shipped ON dbo.Orders(shipped_at);
GO
CREATE VIEW dbo.ActiveUsers AS SELECT id, name, email FROM dbo.Users WHERE email IS NOT NULL;
GO
CREATE PROCEDURE dbo.GetUser @id INT AS SELECT id, name, email FROM dbo.Users WHERE id = @id;
GO
CREATE FUNCTION dbo.fn_FormatName(@n NVARCHAR(50)) RETURNS NVARCHAR(60) AS
BEGIN RETURN UPPER(@n) END;
GO
CREATE PROCEDURE dbo.NewProc AS SELECT 1;
GO
CREATE TYPE dbo.EmailType FROM NVARCHAR(200) NULL;
GO
