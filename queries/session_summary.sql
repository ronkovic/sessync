-- セッション概要
-- 日別のセッション数とメッセージ数を集計
-- PROJECT_ID.DATASET.TABLE を実際のテーブル名に置き換えてください

SELECT
  DATE(timestamp) AS date,
  COUNT(DISTINCT session_id) AS total_sessions,
  COUNT(*) AS total_messages,
  COUNT(CASE WHEN type = 'user' THEN 1 END) AS user_messages,
  COUNT(CASE WHEN type = 'assistant' THEN 1 END) AS assistant_messages,
  ROUND(
    COUNT(CASE WHEN type = 'assistant' THEN 1 END) * 1.0 /
    NULLIF(COUNT(CASE WHEN type = 'user' THEN 1 END), 0),
    2
  ) AS assistant_to_user_ratio,
  COUNT(DISTINCT developer_id) AS active_developers
FROM
  `PROJECT_ID.DATASET.TABLE`
WHERE
  timestamp >= TIMESTAMP_SUB(CURRENT_TIMESTAMP(), INTERVAL 30 DAY)
GROUP BY
  date
ORDER BY
  date DESC;
