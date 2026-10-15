// N3485 12.8p28, 8.5p15: a by-value assignment parameter is
// copy-initialized, so its explicit copy constructor is excluded; expect reject
struct Member
{
  Member() {}
  explicit Member(const Member&) {}
  Member& operator=(Member) { return *this; }
};
struct Holder { Member member; };
int main() { Holder a, b; a = b; }
