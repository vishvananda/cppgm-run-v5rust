const char * const names[3] = {"Jan", "Feb"};

int main()
{
  return names[0][0] == 'J' && names[2] == nullptr ? 0 : 1;
}
