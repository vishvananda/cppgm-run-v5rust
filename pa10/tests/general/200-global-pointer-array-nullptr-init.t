static const char * const paths[] = {
  "a",
  nullptr
};

int main()
{
  return paths[0][0] == 'a' && paths[1] == nullptr ? 0 : 1;
}
