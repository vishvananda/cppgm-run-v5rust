struct Entity
{
};

typedef const Entity *Key;

struct Values
{
  int operator[](const Key&)
  {
    return 1;
  }

  int operator[](Key&&)
  {
    return 2;
  }
};

Entity *pointer()
{
  return 0;
}

int main()
{
  Values values;
  return values[pointer()] == 2 ? 0 : 1;
}
