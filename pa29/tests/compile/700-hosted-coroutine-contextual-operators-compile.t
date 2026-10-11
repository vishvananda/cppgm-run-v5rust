template <typename T>
T hosted_coroutine_contextual_parse(T t)
{
  auto x = co_await t;
  co_yield x;
  co_return x;
}

int co_await;
int co_return;
int co_yield;

int ordinary_identifier_use()
{
  int x = co_await + 1;
  co_return = x;
  co_yield = co_return + 1;
  return co_yield;
}

int main()
{
  return ordinary_identifier_use() == 2 ? 0 : 1;
}
