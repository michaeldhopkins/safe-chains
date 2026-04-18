# Kafka

### `kafka-console-consumer`
<p class="cmd-url"><a href="https://kafka.apache.org/documentation/#basic_ops_consumer">https://kafka.apache.org/documentation/#basic_ops_consumer</a></p>

- Allowed standalone flags: --help, -h, --version, --from-beginning, --skip-message-on-error, --enable-systest-events
- Allowed valued flags: --bootstrap-server, --consumer-property, --consumer.config, --consumer-config, --formatter, --formatter-config, --property, --group, --isolation-level, --key-deserializer, --value-deserializer, --max-messages, --timeout-ms, --offset, --partition, --topic, --include, --whitelist

### `kafka-consumer-groups`
<p class="cmd-url"><a href="https://kafka.apache.org/documentation/#basic_ops_consumer_group">https://kafka.apache.org/documentation/#basic_ops_consumer_group</a></p>

- **--describe**: Flags: --help, -h, --all-groups, --members, --offsets, --state, --verbose. Valued: --bootstrap-server, --command-config, --group, --timeout
- **--list**: Flags: --help, -h. Valued: --bootstrap-server, --command-config, --state
- Allowed standalone flags: --help, -h, --version

### `kafka-topics`
<p class="cmd-url"><a href="https://kafka.apache.org/documentation/#basic_ops_add_topic">https://kafka.apache.org/documentation/#basic_ops_add_topic</a></p>

- **--describe**: Flags: --help, -h, --unavailable-partitions, --under-replicated-partitions, --under-min-isr-partitions, --at-min-isr-partitions, --exclude-internal. Valued: --bootstrap-server, --command-config, --topic, --topic-id, --topic-id-type, --exclude-topic, --partition
- **--list**: Flags: --help, -h, --exclude-internal. Valued: --bootstrap-server, --command-config, --topic, --topic-id, --topic-id-type, --exclude-topic
- Allowed standalone flags: --help, -h, --version

