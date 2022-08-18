module github.com/clintjedwards/gofer/sdk/go

replace github.com/clintjedwards/gofer => ../../

go 1.19

require (
	github.com/clintjedwards/gofer v0.0.0-00010101000000-000000000000
	github.com/google/go-cmp v0.5.6
	github.com/grpc-ecosystem/go-grpc-middleware v1.3.0
	github.com/kelseyhightower/envconfig v1.4.0
	github.com/rs/zerolog v1.27.0
	google.golang.org/grpc v1.48.0
)

require (
	github.com/golang/protobuf v1.5.2 // indirect
	github.com/mattn/go-colorable v0.1.12 // indirect
	github.com/mattn/go-isatty v0.0.14 // indirect
	golang.org/x/net v0.0.0-20220812174116-3211cb980234 // indirect
	golang.org/x/sys v0.0.0-20220811171246-fbc7d0a398ab // indirect
	golang.org/x/text v0.3.7 // indirect
	google.golang.org/genproto v0.0.0-20220812140447-cec7f5303424 // indirect
	google.golang.org/protobuf v1.28.1 // indirect
)