// expect reject
// A completed friend class does not grant access to a later unrelated class.
struct Vault {
private:
  struct Base {};
  friend struct Friend;
};
struct Friend : Vault::Base {};
struct Stranger : Vault::Base {};
