-- ツール使用分析
-- アシスタントメッセージ内のツール使用頻度とパターンを分析
-- PROJECT_ID.DATASET.TABLE を実際のテーブル名に置き換えてください

WITH tool_extracts AS (
  SELECT
    timestamp,
    session_id,
    developer_id,
    -- メッセージJSONからツール名を抽出
    REGEXP_EXTRACT_ALL(
      TO_JSON_STRING(message),
      r'"name"\s*:\s*"([^"]+)"'
    ) AS tools_used
  FROM
    `PROJECT_ID.DATASET.TABLE`
  WHERE
    type = 'assistant'
    AND timestamp >= TIMESTAMP_SUB(CURRENT_TIMESTAMP(), INTERVAL 30 DAY)
),
flattened_tools AS (
  SELECT
    timestamp,
    session_id,
    developer_id,
    tool
  FROM
    tool_extracts,
    UNNEST(tools_used) AS tool
)
SELECT
  tool AS tool_name,
  COUNT(*) AS usage_count,
  COUNT(DISTINCT session_id) AS sessions_using,
  COUNT(DISTINCT developer_id) AS developers_using,
  ROUND(COUNT(*) * 100.0 / SUM(COUNT(*)) OVER(), 2) AS usage_percentage
FROM
  flattened_tools
GROUP BY
  tool
ORDER BY
  usage_count DESC
LIMIT 50;
