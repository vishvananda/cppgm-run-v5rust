struct Poly
{
	virtual int tag() { return 7; }
	long value;
};

extern double our_take_poly(Poly p);

double host_take_poly(Poly p)
{
	return double(p.tag()) + double(p.value);
}

extern "C" int host_drive()
{
	Poly p;
	p.value = 35;
	if (our_take_poly(p) != 42.0) return 3;
	if (host_take_poly(p) != 42.0) return 4;
	return 0;
}
