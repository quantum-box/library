-- PLT-947: inspect pending webhook event rows that can break the worker batch.
-- Read-only by default. Run against the library-api production database.

SELECT
  id,
  endpoint_id,
  provider,
  event_type,
  processing_status,
  retry_count,
  next_retry_at,
  JSON_TYPE(payload) AS payload_type,
  JSON_TYPE(headers) AS headers_type,
  JSON_TYPE(stats) AS stats_type,
  received_at,
  processed_at,
  error_message
FROM webhook_events
WHERE processing_status = 'pending'
  AND (next_retry_at IS NULL OR next_retry_at <= NOW())
ORDER BY received_at ASC
LIMIT 50;

SELECT
  id,
  provider,
  event_type,
  processing_status,
  retry_count,
  received_at,
  error_message
FROM webhook_events
WHERE processing_status = 'pending'
  AND (
    provider NOT IN (
      'github',
      'linear',
      'hubspot',
      'stripe',
      'square',
      'notion',
      'airtable',
      'generic'
    )
    OR retry_count < 0
  )
ORDER BY received_at ASC
LIMIT 50;

-- If a row is confirmed corrupt and blocks the worker repeatedly, archive it
-- manually with an audited UPDATE similar to the statement below.
--
-- UPDATE webhook_events
-- SET processing_status = 'failed',
--     error_message = 'PLT-947 manual quarantine: invalid pending row',
--     processed_at = NOW()
-- WHERE id = '<webhook_event_id>'
--   AND processing_status = 'pending';
