// lowir.md inline_hint: a function states inline_hint=yes exactly when it is an
// inline function (7.1.2): declared inline, defined in its class (9.3p2, a
// friend's too, 11.3p7), or an implicitly declared special member (here a
// destructor); a member defined outside its class without inline is not.
struct Member
{
  int value;
  ~Member() { value = 0; }
};

struct Box
{
  int value;
  Member member;
  int in_class() const { return value + 1; }
  static int static_in_class(int x) { return x + 2; }
  int out_of_class() const;
  inline int inline_out_of_class() const;
  friend int friend_in_class(const Box& box) { return box.value + 3; }
};

int Box::out_of_class() const { return value + 4; }
int Box::inline_out_of_class() const { return value + 5; }
inline int inline_function(int x) { return x + 6; }
int ordinary_function(int x) { return x + 7; }

int main()
{
  Box box;
  box.value = 0;
  box.member.value = 1;
  return box.in_class() + Box::static_in_class(0) + box.out_of_class() +
    box.inline_out_of_class() + friend_in_class(box) + inline_function(0) +
    ordinary_function(0) == 28 ? 0 : 1;
}
