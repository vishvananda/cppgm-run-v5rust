namespace boost {
__extension__ typedef __int128 int128_type;
__extension__ typedef unsigned __int128 uint128_type;
}

static_assert(sizeof(boost::int128_type) == 16, "signed int128 size");
static_assert(sizeof(boost::uint128_type) == 16, "unsigned int128 size");

boost::int128_type signed_value;
boost::uint128_type unsigned_value;
