global @ro_value : i64 [storage=readonly] = 7
global @ro_table [storage=readonly] = {
  i8 65
  zero 7
}

function @main() -> i64 [role=entry] {
  block ^entry:
    %0 = load i64 @ro_value
    return i64 %0
}
