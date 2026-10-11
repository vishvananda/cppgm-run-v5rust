struct Item
{
  Item() {}
};

thread_local Item tls;
int tls__tls_guard = 7;

void tls__tls_init() {}

int main()
{
  return tls__tls_guard - 7;
}
