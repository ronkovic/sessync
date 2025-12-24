-- エラーパターン分析
-- セッション内のエラーパターンを検出・分析
-- PROJECT_ID.DATASET.TABLE を実際のテーブル名に置き換えてください

-- エラーインジケータを含むメッセージを抽出
WITH error_messages AS (
  SELECT
    timestamp,
    session_id,
    developer_id,
    type,
    message,
    CASE
      WHEN LOWER(TO_JSON_STRING(message)) LIKE '%error%' THEN 'error'
      WHEN LOWER(TO_JSON_STRING(message)) LIKE '%failed%' THEN 'failed'
      WHEN LOWER(TO_JSON_STRING(message)) LIKE '%exception%' THEN 'exception'
      WHEN LOWER(TO_JSON_STRING(message)) LIKE '%warning%' THEN 'warning'
      ELSE 'other'
    END AS error_type
  FROM
    `PROJECT_ID.DATASET.TABLE`
  WHERE
    timestamp >= TIMESTAMP_SUB(CURRENT_TIMESTAMP(), INTERVAL 30 DAY)
    AND (
      LOWER(TO_JSON_STRING(message)) LIKE '%error%'
      OR LOWER(TO_JSON_STRING(message)) LIKE '%failed%'
      OR LOWER(TO_JSON_STRING(message)) LIKE '%exception%'
      OR LOWER(TO_JSON_STRING(message)) LIKE '%warning%'
    )
)
SELECT
  DATE(timestamp) AS date,
  error_type,
  COUNT(*) AS error_count,
  COUNT(DISTINCT session_id) AS affected_sessions
FROM
  error_messages
GROUP BY
  date,
  error_type
ORDER BY
  date DESC,
  error_count DESC;

-- 高エラー率のセッション（コメント解除して使用）
-- SELECT
--   session_id,
--   developer_id,
--   COUNT(*) AS total_messages,
--   SUM(CASE
--     WHEN LOWER(TO_JSON_STRING(message)) LIKE '%error%'
--       OR LOWER(TO_JSON_STRING(message)) LIKE '%failed%'
--     THEN 1 ELSE 0
--   END) AS error_messages,
--   ROUND(
--     SUM(CASE
--       WHEN LOWER(TO_JSON_STRING(message)) LIKE '%error%'
--         OR LOWER(TO_JSON_STRING(message)) LIKE '%failed%'
--       THEN 1 ELSE 0
--     END) * 100.0 / COUNT(*),
--     2
--   ) AS error_rate_percent
-- FROM
--   `PROJECT_ID.DATASET.TABLE`
-- WHERE
--   timestamp >= TIMESTAMP_SUB(CURRENT_TIMESTAMP(), INTERVAL 7 DAY)
-- GROUP BY
--   session_id,
--   developer_id
-- HAVING
--   error_rate_percent > 10
-- ORDER BY
--   error_rate_percent DESC;
