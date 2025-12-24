-- メッセージ分析
-- メッセージタイプの分布と特性を分析
-- PROJECT_ID.DATASET.TABLE を実際のテーブル名に置き換えてください

-- メッセージタイプ別の分布
SELECT
  type AS message_type,
  COUNT(*) AS count,
  ROUND(COUNT(*) * 100.0 / SUM(COUNT(*)) OVER(), 2) AS percentage
FROM
  `PROJECT_ID.DATASET.TABLE`
WHERE
  timestamp >= TIMESTAMP_SUB(CURRENT_TIMESTAMP(), INTERVAL 30 DAY)
GROUP BY
  type
ORDER BY
  count DESC;

-- セッションあたりの平均メッセージ数（コメント解除して使用）
-- SELECT
--   AVG(message_count) AS avg_messages_per_session,
--   MIN(message_count) AS min_messages,
--   MAX(message_count) AS max_messages,
--   APPROX_QUANTILES(message_count, 100)[OFFSET(50)] AS median_messages
-- FROM (
--   SELECT
--     session_id,
--     COUNT(*) AS message_count
--   FROM
--     `PROJECT_ID.DATASET.TABLE`
--   WHERE
--     timestamp >= TIMESTAMP_SUB(CURRENT_TIMESTAMP(), INTERVAL 30 DAY)
--   GROUP BY
--     session_id
-- );

-- セッション時間分析（コメント解除して使用）
-- SELECT
--   session_id,
--   MIN(timestamp) AS session_start,
--   MAX(timestamp) AS session_end,
--   TIMESTAMP_DIFF(MAX(timestamp), MIN(timestamp), MINUTE) AS duration_minutes,
--   COUNT(*) AS message_count
-- FROM
--   `PROJECT_ID.DATASET.TABLE`
-- WHERE
--   timestamp >= TIMESTAMP_SUB(CURRENT_TIMESTAMP(), INTERVAL 7 DAY)
-- GROUP BY
--   session_id
-- ORDER BY
--   duration_minutes DESC
-- LIMIT 100;
