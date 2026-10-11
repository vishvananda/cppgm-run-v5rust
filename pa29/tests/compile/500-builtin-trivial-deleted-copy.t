struct DeletedCopy
{
  DeletedCopy() = default;
  DeletedCopy(const DeletedCopy&) = delete;
  DeletedCopy& operator=(const DeletedCopy&) = delete;
};

static_assert(__is_trivially_copyable(DeletedCopy), "deleted copy declaration is trivial");
static_assert(__is_trivial(DeletedCopy), "trivial classes are trivially copyable");
static_assert(__is_pod(DeletedCopy), "POD classes are trivial");

int main()
{
  return 0;
}
