-- 開発者統計
-- 開発者ごとの生産性とアクティビティ指標
-- PROJECT_ID.DATASET.TABLE を実際のテーブル名に置き換えてください

SELECT
  developer_id,
  user_email,
  COUNT(DISTINCT DATE(timestamp)) AS active_days,
  COUNT(DISTINCT session_id) AS total_sessions,
  COUNT(*) AS total_messages,
  COUNT(CASE WHEN type = 'user' THEN 1 END) AS user_messages,
  COUNT(CASE WHEN type = 'assistant' THEN 1 END) AS assistant_messages,
  ROUND(
    COUNT(*) * 1.0 / COUNT(DISTINCT DATE(timestamp)),
    1
  ) AS avg_messages_per_day,
  ROUND(
    COUNT(DISTINCT session_id) * 1.0 / COUNT(DISTINCT DATE(timestamp)),
    1
  ) AS avg_sessions_per_day,
  MIN(timestamp) AS first_activity,
  MAX(timestamp) AS last_activity
FROM
  `PROJECT_ID.DATASET.TABLE`
WHERE
  timestamp >= TIMESTAMP_SUB(CURRENT_TIMESTAMP(), INTERVAL 30 DAY)
GROUP BY
  developer_id,
  user_email
ORDER BY
  total_messages DESC;
