// expect exit 0
// N3485 7.1.6.2p4, 11p6: selected expression access waits for the class;
// an unparenthesized data-member operand retains its declared type.
struct Base { int value; };
struct Vault {
private:
  Base member;
  static Base make();
  static Base make(int);
  Base operator+();
  friend struct Friend;
};
extern Vault vault;
Base select(Base (*)());
struct Friend {
  struct FromMember : decltype(vault.member) {};
  struct FromAddress : decltype(select(&Vault::make)) {};
  struct FromOperator : decltype(+vault) {};
};
struct Secret {
private:
  Secret();
  ~Secret();
  friend struct Derived;
};
struct Derived : decltype(Secret()) {};
int main() {
  Friend::FromMember first;
  Friend::FromAddress second;
  Friend::FromOperator third;
  first.value = 1;
  second.value = 2;
  third.value = 3;
  return first.value + second.value + third.value == 6 ? 0 : 1;
}
