// VALIDATION: compile-pass
// N3485 focus: 14.7.2 [temp.explicit]

template<typename T>
struct box
{
  int early() const;
  int late() const;
};

template<typename T>
int box<T>::early() const
{
  return 1;
}

template struct box<int>;

template<typename T>
int box<T>::late() const
{
  return 2;
}

int main()
{
  return 0;
}
