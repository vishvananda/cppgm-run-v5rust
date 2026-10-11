template<class T> struct Box { T value; };
static_assert(sizeof(Box<int>) == sizeof(int), "");
