int main()
{
  const char * value = "abc" "def" "ghij";
  return value[0] == 'a' &&
                 value[3] == 'd' &&
                 value[6] == 'g' &&
                 value[9] == 'j' &&
                 value[10] == '\0' ?
             0 :
             12;
}
