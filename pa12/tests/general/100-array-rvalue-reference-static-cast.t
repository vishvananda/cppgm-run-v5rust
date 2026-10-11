typedef char chars[2];

int main()
{
  const chars && value = static_cast<const chars &&>("x");
  return value[0] == 'x' ? 0 : 1;
}

// VALIDATION: compile-pass
