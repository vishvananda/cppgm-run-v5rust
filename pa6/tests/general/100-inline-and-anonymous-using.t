inline namespace A
{
	using T = int;
};

T i;
namespace A {}

inline namespace
{
	using U = double;
}

U d;
