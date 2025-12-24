-- 日別アクティビティヒートマップ
-- 曜日×時間帯のメッセージ数を集計（ヒートマップ用データ）
-- PROJECT_ID.DATASET.TABLE を実際のテーブル名に置き換えてください

SELECT
  FORMAT_DATE('%A', DATE(timestamp)) AS day_of_week,
  EXTRACT(DAYOFWEEK FROM timestamp) AS day_number,
  EXTRACT(HOUR FROM timestamp) AS hour,
  COUNT(*) AS message_count,
  COUNT(DISTINCT session_id) AS session_count
FROM
  `PROJECT_ID.DATASET.TABLE`
WHERE
  timestamp >= TIMESTAMP_SUB(CURRENT_TIMESTAMP(), INTERVAL 30 DAY)
GROUP BY
  day_of_week,
  day_number,
  hour
ORDER BY
  day_number,
  hour;
