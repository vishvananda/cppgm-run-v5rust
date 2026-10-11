// N3485 focus: 13.3.1.2 [over.match.oper], built-in operator candidates
// remain available when a found overloaded comma candidate is not viable.

namespace probe {

struct tag {};

struct incrementable {
  int value;

  incrementable() : value(0) {}

  incrementable & operator++()
  {
    value = 3;
    return *this;
  }
};

tag operator,(tag, int);

int run()
{
  incrementable x;
  int result = (++x, 7);
  return x.value == 3 && result == 7 ? 0 : 1;
}

}

int main()
{
  return probe::run();
}
