// expect exit 0
// N3485 11p6-p7, 11.3p2: base-clause lookup uses the enclosing scope;
// access uses the declared class after every base edge has been formed.
struct Vault {
private:
  struct Base { int value; };
  typedef Base Alias;
  static Base make();
  friend struct Friend;
};
struct Friend : Vault::Alias {
  struct Nested : Vault::Base {};
  struct FromCall : decltype(Vault::make()) {};
  static int local() {
    struct Local : Vault::Base {};
    Local object;
    object.value = 7;
    return object.value;
  }
};
int main() {
  Friend object;
  Friend::Nested nested;
  Friend::FromCall from_call;
  object.value = 1;
  nested.value = 2;
  from_call.value = 3;
  return object.value + nested.value + from_call.value + Friend::local() == 13 ? 0 : 1;
}
