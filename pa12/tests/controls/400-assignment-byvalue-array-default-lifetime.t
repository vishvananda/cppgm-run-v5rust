// N3485 12.8p28, 8.5p15, 5.2.2p4: each assigned element owns its
// parameter copy; its constructor defaults end before operator=; expect exit 0
int alive;
int defaults;
int copies;
int assignments;
int bad;

struct Default
{
  Default() { ++defaults; }
  ~Default() { --defaults; }
};

struct Member
{
  int value;
  Member() : value(7) { ++alive; }
  Member(const Member& other, const Default& = Default()) : value(other.value)
  {
    ++alive;
    ++copies;
    if (defaults != 1) ++bad;
  }
  Member& operator=(Member other)
  {
    ++assignments;
    if (defaults != 0) ++bad;
    value = other.value;
    return *this;
  }
  ~Member() { value = 99; --alive; }
};

struct Small { Member members[3]; };
struct Large { Member members[2][5]; };

int main()
{
  {
    Small a, b;
    Large c, d;
    b.members[1].value = 11;
    d.members[1][4].value = 13;
    a = b;
    c = d;
    if (alive != 26 || copies != 13 || assignments != 13 || defaults || bad) return 1;
    if (a.members[1].value != 11 || b.members[1].value != 11) return 2;
    if (c.members[1][4].value != 13 || d.members[1][4].value != 13) return 3;
  }
  return alive || defaults || bad;
}
