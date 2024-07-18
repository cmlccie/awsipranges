Require awsipranges
Output awsipranges.gif

Set FontSize 20
Set Width 1280
Set Height 640

Type "# Lookup addresses"
Sleep 500ms
Enter
Type "awsipranges 35.72.164.242"
Sleep 500ms
Enter
Sleep 2s
Type "awsipranges 2600:1f1a:4000:a03a:54b4:19e6:f50c:9d01"
Sleep 500ms
Enter
Sleep 4s

Type "clear"
Sleep 500ms
Enter


Type "# Get IP ranges for a service"
Sleep 500ms
Enter
Type "awsipranges --service S3 --ipv4"
Sleep 500ms
Enter
Sleep 4s

Type "clear"
Sleep 500ms
Enter


Type "# Get IPv4 or IPv6 prefixes and filter by region"
Sleep 500ms
Enter
Type "awsipranges --ipv6 --service s3 --region eu-central-1"
Sleep 500ms
Enter
Sleep 4s

Type "clear"
Sleep 500ms
Enter


Type "# Identify network border groups"
Sleep 500ms
Enter
Type "awsipranges --output network-border-groups --region us-west-2"
Sleep 500ms
Enter
Sleep 2s
Type "# Get IP ranges for a Local Zone"
Sleep 500ms
Enter
Type "awsipranges --network-border-group us-west-2-lax-1"
Enter
Sleep 4s
Type "# ...and more!"
Sleep 500ms
Enter
Sleep 4s

Type "clear"
Sleep 500ms
Enter
