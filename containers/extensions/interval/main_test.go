package main

// func TestNewExtension(t *testing.T) {
// 	extension := newExtension()

// 	fmt.Println(len(extension.events))
// 	go extension.Check(&sdkProto.CheckRequest{})

// 	ctx, cancel := context.WithCancel(context.Background())
// 	go extension.startInterval(ctx, "testPipeline", "1", time.Second)
// 	time.Sleep(time.Second * 2)

// 	cancel()
// 	//quit <- true

// 	//fmt.Println(len(extension.events))
// 	//fmt.Println(extension.events)
// }

// func TestNewSubscribe(t *testing.T) {
// 	extension := newExtension()
// 	_, err := extension.Subscribe(&sdkProto.SubscribeRequest{
// 		Id: "1",
// 		Config: map[string]string{
// 			"every": "1s",
// 		},
// 	})
// 	if err != nil {
// 		t.Fatal(err)
// 	}

// 	//fmt.Println(resp)
// }

// func TestNewUnsubscribe(t *testing.T) {
// 	extension := newExtension()
// 	_, err := extension.Subscribe(&sdkProto.SubscribeRequest{
// 		Id: "1",
// 		Config: map[string]string{
// 			"every": "1s",
// 		},
// 	})
// 	if err != nil {
// 		t.Fatal(err)
// 	}

// 	_, err = extension.Unsubscribe(&sdkProto.UnsubscribeRequest{
// 		Id: "1",
// 	})
// 	if err != nil {
// 		t.Fatal(err)
// 	}

// 	time.Sleep(time.Second * 2)

// 	fmt.Println(extension.subscriptions)
// }
