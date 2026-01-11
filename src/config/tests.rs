#[cfg(test)]
mod tests {
    use super::super::parser::parse_config;
    use std::path::Path;

    #[test]
    fn test_simple_config() {
        let config_str = r#"
            server {
                listen 8080;
                server_name localhost;
                root /var/www/html;
                index index.html;
            }
        "#;
        let config = parse_config(config_str, Path::new(".")).unwrap();
        assert_eq!(config.servers.len(), 1);
        let s = &config.servers[0];
        assert_eq!(s.listen[0].port(), 8080);
        assert_eq!(s.server_names[0], "localhost");
    }

    #[test]
    fn test_multiple_servers() {
        let config_str = r#"
            server {
                listen 8000;
            }
            server {
                listen 9000;
            }
        "#;
        let config = parse_config(config_str, Path::new(".")).unwrap();
        assert_eq!(config.servers.len(), 2);
    }

    #[test]
    fn test_comments() {
        let config_str = r#"
            # This is a comment
            server {
                listen 8080; # Inline comment
            }
        "#;
        let config = parse_config(config_str, Path::new(".")).unwrap();
        assert_eq!(config.servers.len(), 1);
    }

    #[test]
    fn test_location_block() {
        let config_str = r#"
            server {
                listen 8080;
                location / {
                    root /www;
                    methods GET POST;
                    autoindex on;
                }
            }
        "#;
        let config = parse_config(config_str, Path::new(".")).unwrap();
        let s = &config.servers[0];
        assert_eq!(s.locations.len(), 1);
        let loc = &s.locations[0];
        assert_eq!(loc.path, "/");
        assert_eq!(loc.autoindex, Some(true));
    }
}
