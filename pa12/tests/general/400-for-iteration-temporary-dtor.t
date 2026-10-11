int destroyed;

struct object
{
  ~object() { ++destroyed; }
  object& operator=(const object&) { return *this; }
};

object make() { return object(); }

int main()
{
  object target;
  int count = 0;
  for(; count != 2; target = make(), ++count) {
    if(destroyed != count) return 1;
  }
  return destroyed == 2 ? 0 : 2;
}
