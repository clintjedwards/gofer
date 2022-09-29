package main

// func TestNewTrigger(t *testing.T) {
// 	trigger := newTrigger()

// 	fmt.Println(len(trigger.events))
// 	go trigger.Check(&sdkProto.CheckRequest{})

// 	ctx, cancel := context.WithCancel(context.Background())
// 	go trigger.startInterval(ctx, "testPipeline", "1", time.Second)
// 	time.Sleep(time.Second * 2)

// 	cancel()
// 	//quit <- true

// 	//fmt.Println(len(trigger.events))
// 	//fmt.Println(trigger.events)
// }

// func TestNewSubscribe(t *testing.T) {
// 	trigger := newTrigger()
// 	_, err := trigger.Subscribe(&sdkProto.SubscribeRequest{
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
// 	trigger := newTrigger()
// 	_, err := trigger.Subscribe(&sdkProto.SubscribeRequest{
// 		Id: "1",
// 		Config: map[string]string{
// 			"every": "1s",
// 		},
// 	})
// 	if err != nil {
// 		t.Fatal(err)
// 	}

// 	_, err = trigger.Unsubscribe(&sdkProto.UnsubscribeRequest{
// 		Id: "1",
// 	})
// 	if err != nil {
// 		t.Fatal(err)
// 	}

// 	time.Sleep(time.Second * 2)

// 	fmt.Println(trigger.subscriptions)
// }
