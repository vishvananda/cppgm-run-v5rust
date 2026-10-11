namespace std { template<typename T> class initializer_list; }

struct Item {};

struct Box {
  Box();
  Box(const Box&);
  Box(Box&&);
  Box(std::initializer_list<Item>);

  Box& operator=(const Box&);
  Box& operator=(Box&&);
  Box& operator=(std::initializer_list<Item>);
};

void f(Box& b, Item item)
{
  b = {item};
}
