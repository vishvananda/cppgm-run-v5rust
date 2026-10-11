int created, drops, bad, next = 3;

struct G
{
  int id;
  G() : id(++created) {}
  ~G() noexcept(false)
  {
    if (id != next--) bad = 1;
    ++drops;
    if (id == 2) throw 7;
  }
};

void f() { G objects[3]; }

int main()
{
  try { f(); return 1; }
  catch (int value)
  {
    return value != 7 || bad || next || drops != 3;
  }
}
