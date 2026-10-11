int probe();
struct holder
{
  // A nondependent invalid expression is checked without using the member.
  template<class T> void unused() noexcept(probe() > 0);
};

int main() { return 0; }
