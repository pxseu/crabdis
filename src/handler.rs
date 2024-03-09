// pub async fn parse_command(
//     command: String,
//     store: &Arc<AsyncRwLock<HashMap<String, ValueWithTTL>>>,
// ) -> String {
//     let parts: Vec<&str> = command.split_whitespace().collect();
//     match parts.as_slice() {
//         ["SET", key, value] => {
//             store
//                 .write()
//                 .await
//                 .insert(key.to_string(), (value.to_string(), None));
//             "+OK\r\n".to_string() // RESP Simple String
//         }
//         ["SETEX", key, seconds, value] => {
//             let ttl = seconds
//                 .parse::<u64>()
//                 .ok()
//                 .map(|secs| Instant::now() + Duration::new(secs, 0));
//             store
//                 .write()
//                 .await
//                 .insert(key.to_string(), (value.to_string(), ttl));
//             "+OK\r\n".to_string() // RESP Simple String
//         }
//         ["GET", key] => {
//             let store = store.read().await;
//             match store.get(key) {
//                 Some((value, Some(expiry))) if Instant::now() > *expiry => {
//                     "$-1\r\n".to_string() // RESP Bulk String for expired key
//                 }
//                 Some((value, _)) => format!("${}\r\n{}\r\n", value.len(), value), // RESP Bulk String
//                 None => "$-1\r\n".to_string(), // RESP Bulk String for nil
//             }
//         }
//         // Additional handling for MGET to consider TTL
//         [command @ "MGET", keys @ ..] if command == &"MGET" => {
//             let store = store.read().await;
//             let values: Vec<String> = keys
//                 .iter()
//                 .map(|&key| match store.get(key) {
//                     Some((value, Some(expiry))) if Instant::now() > *expiry => {
//                         "$-1\r\n".to_string()
//                     }
//                     Some((value, _)) => format!("${}\r\n{}\r\n", value.len(), value),
//                     None => "$-1\r\n".to_string(),
//                 })
//                 .collect();
//             format!("*{}\r\n{}", values.len(), values.join("")) // RESP Array
//         }
//         _ => "-Error: Invalid command\r\n".to_string(), // RESP Error
//     }
// }
