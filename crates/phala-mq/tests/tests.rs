use phala_mq::Origin;

#[cfg(feature = "queue")]
#[test]
fn test_send_message() {
    struct TestSigner(Vec<u8>);

    impl MessageSigner for TestSigner {
        fn sign(&self, _data: &[u8]) -> Vec<u8> {
            self.0.clone()
        }
    }

    use phala_mq::{MessageSendQueue, MessageSigner};
    let queue = MessageSendQueue::new();
    let runtime = Origin::Pallet(b"p0".to_vec());
    let worker0 = Origin::Worker(b"worker0".to_vec());

    {
        let signer = TestSigner(b"key0".to_vec());

        let handle00 = queue.channel(runtime.clone(), signer);

        handle00.send(b"payload00".to_vec(), b"phala.network/test0".to_vec());

        let signer = TestSigner(b"key1".to_vec());

        let handle01 = queue.channel(runtime.clone(), signer);
        handle01.send(b"payload01".to_vec(), b"phala.network/test1".to_vec());

        handle00.send(b"payload02".to_vec(), b"phala.network/test1".to_vec());

        let messages = queue.all_messages();

        assert_eq!(messages.len(), 3);

        assert_eq!(messages[0].message.sender, runtime);
        assert_eq!(messages[0].sequence, 0);
        assert_eq!(messages[0].signature, b"key0");

        assert_eq!(messages[1].message.sender, runtime);
        assert_eq!(messages[1].sequence, 1);
        assert_eq!(messages[1].signature, b"key1");

        assert_eq!(messages[2].message.sender, runtime);
        assert_eq!(messages[2].sequence, 2);
        assert_eq!(messages[2].signature, b"key0");
    }

    {
        let signer = TestSigner(b"a key".to_vec());
        let handle = queue.channel(worker0.clone(), signer);

        handle.send(b"energy".to_vec(), b"/the/hole".to_vec());
        handle.send(b"energy".to_vec(), b"/the/hole".to_vec());
        handle.send(b"energy".to_vec(), b"/the/hole".to_vec());

        assert_eq!(queue.messages(&worker0).len(), 3);
    }

    {
        queue.purge(|sender| match &sender {
            Origin::Pallet(_) => 1,
            _ => 0,
        });

        let runtime_msgs = queue.messages(&runtime);
        let contract1_msgs = queue.messages(&worker0);

        assert_eq!(runtime_msgs.len(), 2);
        assert_eq!(contract1_msgs.len(), 3);
    }
}

#[cfg(feature = "dispatcher")]
#[test]
fn test_dispatcher() {
    use phala_mq::{Message, MessageDispatcher};
    let sender0 = Origin::Pallet(b"sender0".to_vec());
    let sender1 = Origin::Pallet(b"sender1".to_vec());

    let mut dispatcher = MessageDispatcher::new();

    let mut sub0 = dispatcher.subscribe(*b"path0");
    let mut sub1 = dispatcher.subscribe(*b"path1");

    let n = dispatcher.dispatch(Message::new(sender0.clone(), *b"path0", b"payload0".to_vec()));
    assert_eq!(n, 1);

    let mut sub2 = dispatcher.subscribe(*b"path0");
    let n = dispatcher.dispatch(Message::new(sender0.clone(), *b"path1", b"payload1".to_vec()));
    assert_eq!(n, 1);
    let _ = dispatcher.dispatch(Message::new(sender1.clone(), *b"path1", b"payload2".to_vec()));
    let n = dispatcher.dispatch(Message::new(sender1.clone(), *b"path0", b"payload3".to_vec()));
    assert_eq!(n, 2);

    {
        let msgs: Vec<Message> = sub0.drain().collect();
        assert_eq!(msgs.len(), 2);
        assert_eq!(&msgs[0].sender, &sender0);
        assert_eq!(msgs[0].destination.path(), b"path0");
        assert_eq!(msgs[0].payload, b"payload0");
        assert_eq!(&msgs[1].sender, &sender1);
        assert_eq!(msgs[1].destination.path(), b"path0");
        assert_eq!(msgs[1].payload, b"payload3");
    }
    {
        let msgs: Vec<Message> = sub1.drain().collect();

        assert_eq!(msgs.len(), 2);
        assert_eq!(&msgs[0].sender, &sender0);
        assert_eq!(msgs[0].destination.path(), b"path1");
        assert_eq!(msgs[0].payload, b"payload1");
        assert_eq!(&msgs[1].sender, &sender1);
        assert_eq!(msgs[1].destination.path(), b"path1");
        assert_eq!(msgs[1].payload, b"payload2");
    }
    {
        let msgs: Vec<Message> = sub2.drain().collect();
        assert_eq!(msgs.len(), 1);
        assert_eq!(&msgs[0].sender, &sender1);
        assert_eq!(msgs[0].destination.path(), b"path0");
        assert_eq!(msgs[0].payload, b"payload3");
    }
}
