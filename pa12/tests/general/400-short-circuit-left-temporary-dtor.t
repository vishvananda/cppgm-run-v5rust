int destroyed;

struct object
{
  ~object() { ++destroyed; }
};

object make() { return object(); }
bool operator==(const object&, const object&) { return true; }
bool use_after() { return destroyed == 0; }

int main()
{
  object value;
  if(value == make() || false) {}
  if(destroyed != 1) return 1;

  destroyed = 0;
  if(value == make() && use_after()) {}
  return destroyed == 1 ? 0 : 2;
}
