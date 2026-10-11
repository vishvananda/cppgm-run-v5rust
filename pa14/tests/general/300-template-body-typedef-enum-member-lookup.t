template<class T>
struct box
{
  typedef enum
  {
    white = 0,
    black
  } color_type;

  int value()
  {
    return white;
  }
};

int main()
{
  box<int> b;
  return b.value();
}
