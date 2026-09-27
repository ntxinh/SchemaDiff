CREATE DATABASE schemadiff_src;
GO
USE schemadiff_src;
GO
CREATE TABLE dbo.Users (
    id INT IDENTITY(1,1) NOT NULL,
    name NVARCHAR(50) NOT NULL,
    age INT NULL,
    created_at DATETIME NULL DEFAULT (GETDATE()),
    CONSTRAINT PK_Users PRIMARY KEY (id)
);
GO
CREATE TABLE dbo.Orders (
    id INT IDENTITY(1,1) NOT NULL,
    user_id INT NOT NULL,
    total DECIMAL(18,2) NOT NULL,
    status NVARCHAR(20) NULL,
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
CREATE INDEX IX_Users_Name ON dbo.Users(name);
GO
CREATE VIEW dbo.ActiveUsers AS SELECT id, name FROM dbo.Users;
GO
CREATE PROCEDURE dbo.GetUser @id INT AS SELECT * FROM dbo.Users WHERE id = @id;
GO
CREATE FUNCTION dbo.fn_FormatName(@n NVARCHAR(50)) RETURNS NVARCHAR(60) AS
BEGIN RETURN UPPER(@n) END;
GO
CREATE TRIGGER dbo.trg_Users_Audit ON dbo.Users AFTER INSERT AS SELECT 1;
GO
CREATE TYPE dbo.EmailType FROM NVARCHAR(100) NULL;
GO
