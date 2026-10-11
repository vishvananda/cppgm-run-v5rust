typedef unsigned char syntax_type;

static const syntax_type syntax_char = 0;
static const syntax_type syntax_newline = 26;

static syntax_type char_syntax[] =
{
  syntax_char,
  syntax_newline
};

int main()
{
  return char_syntax[0] == 0 && char_syntax[1] == 26 ? 0 : 1;
}
