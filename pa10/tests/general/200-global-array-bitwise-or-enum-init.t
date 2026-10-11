enum CharClass
{
  char_class_alpha = 96,
  char_class_word = 1024
};

static const unsigned masks[] =
{
  0,
  char_class_alpha | char_class_word
};

int main()
{
  return masks[1] == (96 | 1024) ? 0 : 1;
}
