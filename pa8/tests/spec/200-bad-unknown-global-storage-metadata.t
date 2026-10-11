global @ro_value : i64 [storage=sticky] = 7

function @main() -> i64 [role=entry] {
  block ^entry:
    return i64 0
}
