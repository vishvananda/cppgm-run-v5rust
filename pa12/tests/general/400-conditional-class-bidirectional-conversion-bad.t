struct Right;
struct Left
{
  Left();
  Left(const Right &);
};
struct Right
{
  Right();
  Right(const Left &);
};

void inspect(bool first, const Left & left, const Right & right)
{
  // Both value conversions are viable; the conditional is ambiguous.
  (void)(first ? left : right);
}
