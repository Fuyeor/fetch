-- CreateExtension
CREATE EXTENSION IF NOT EXISTS "citext";

-- CreateEnum
CREATE TYPE "DomainStatus" AS ENUM ('pending', 'verified', 'failed');

-- CreateEnum
CREATE TYPE "SitemapStatus" AS ENUM ('active', 'paused', 'error');

-- CreateEnum
CREATE TYPE "IngestionStatus" AS ENUM ('pending', 'running', 'succeeded', 'failed');

-- CreateTable
CREATE TABLE "user" (
    "id" UUID NOT NULL,
    "username" CITEXT NOT NULL,
    "nickname" VARCHAR(64) NOT NULL,
    "avatar" VARCHAR(255),
    "created_at" TIMESTAMPTZ(6) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMPTZ(6) NOT NULL DEFAULT CURRENT_TIMESTAMP,

    CONSTRAINT "user_pkey" PRIMARY KEY ("id")
);

-- CreateTable
CREATE TABLE "user_refresh_token" (
    "id" UUID NOT NULL,
    "user_id" UUID NOT NULL,
    "token" VARCHAR(512) NOT NULL,
    "expires_at" TIMESTAMPTZ(6) NOT NULL,
    "created_at" TIMESTAMPTZ(6) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "user_agent" TEXT,
    "ip_address" VARCHAR(45),

    CONSTRAINT "user_refresh_token_pkey" PRIMARY KEY ("id")
);

-- CreateTable
CREATE TABLE "search_history" (
    "id" SERIAL NOT NULL,
    "user_id" UUID NOT NULL,
    "query" VARCHAR(255) NOT NULL,
    "created_at" TIMESTAMPTZ(6) NOT NULL DEFAULT CURRENT_TIMESTAMP,

    CONSTRAINT "search_history_pkey" PRIMARY KEY ("id")
);

-- CreateTable
CREATE TABLE "domain" (
    "user_id" UUID NOT NULL,
    "domain" VARCHAR(253) NOT NULL,
    "status" "DomainStatus" NOT NULL DEFAULT 'pending',
    "created_at" TIMESTAMPTZ(6) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "verified_at" TIMESTAMPTZ(6),
    "updated_at" TIMESTAMPTZ(6) NOT NULL DEFAULT CURRENT_TIMESTAMP,

    CONSTRAINT "domain_pkey" PRIMARY KEY ("domain")
);

-- CreateTable
CREATE TABLE "sitemap" (
    "id" UUID NOT NULL,
    "domain" VARCHAR(253) NOT NULL,
    "url" VARCHAR(512) NOT NULL,
    "status" "SitemapStatus" NOT NULL DEFAULT 'active',
    "last_generation" INTEGER,
    "last_ingested_at" TIMESTAMPTZ(6),
    "created_at" TIMESTAMPTZ(6) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMPTZ(6) NOT NULL DEFAULT CURRENT_TIMESTAMP,

    CONSTRAINT "sitemap_pkey" PRIMARY KEY ("id")
);

-- CreateTable
CREATE TABLE "ingestion" (
    "id" UUID NOT NULL,
    "domain" VARCHAR(253) NOT NULL,
    "status" "IngestionStatus" NOT NULL DEFAULT 'pending',
    "generation" INTEGER,
    "documents_added" INTEGER NOT NULL DEFAULT 0,
    "documents_updated" INTEGER NOT NULL DEFAULT 0,
    "documents_deleted" INTEGER NOT NULL DEFAULT 0,
    "created_at" TIMESTAMPTZ(6) NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "finished_at" TIMESTAMPTZ(6),

    CONSTRAINT "ingestion_pkey" PRIMARY KEY ("id")
);

-- CreateIndex
CREATE UNIQUE INDEX "user_username_key" ON "user"("username");

-- CreateIndex
CREATE UNIQUE INDEX "user_refresh_token_token_key" ON "user_refresh_token"("token");

-- CreateIndex
CREATE INDEX "user_refresh_token_user_id_idx" ON "user_refresh_token"("user_id");

-- CreateIndex
CREATE INDEX "search_history_user_id_created_at_idx" ON "search_history"("user_id", "created_at" DESC);

-- CreateIndex
CREATE INDEX "domain_user_id_idx" ON "domain"("user_id");

-- CreateIndex
CREATE INDEX "sitemap_domain_idx" ON "sitemap"("domain");

-- CreateIndex
CREATE INDEX "ingestion_domain_idx" ON "ingestion"("domain");

-- AddForeignKey
ALTER TABLE "user_refresh_token" ADD CONSTRAINT "user_refresh_token_user_id_fkey" FOREIGN KEY ("user_id") REFERENCES "user"("id") ON DELETE CASCADE ON UPDATE NO ACTION;

-- AddForeignKey
ALTER TABLE "search_history" ADD CONSTRAINT "search_history_user_id_fkey" FOREIGN KEY ("user_id") REFERENCES "user"("id") ON DELETE CASCADE ON UPDATE NO ACTION;

-- AddForeignKey
ALTER TABLE "domain" ADD CONSTRAINT "domain_user_id_fkey" FOREIGN KEY ("user_id") REFERENCES "user"("id") ON DELETE CASCADE ON UPDATE NO ACTION;

-- AddForeignKey
ALTER TABLE "sitemap" ADD CONSTRAINT "sitemap_domain_fkey" FOREIGN KEY ("domain") REFERENCES "domain"("domain") ON DELETE CASCADE ON UPDATE NO ACTION;

-- AddForeignKey
ALTER TABLE "ingestion" ADD CONSTRAINT "ingestion_domain_fkey" FOREIGN KEY ("domain") REFERENCES "domain"("domain") ON DELETE CASCADE ON UPDATE NO ACTION;
