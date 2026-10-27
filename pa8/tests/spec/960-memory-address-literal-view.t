function @f()->i64 {
block ^entry:
store i64 -1,-2
%value = load i64 -1
return i64 %value
}
