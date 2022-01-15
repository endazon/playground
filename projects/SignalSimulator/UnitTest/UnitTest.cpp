#include "pch.h"
#include "CppUnitTest.h"
#include "UnitOfNumber.h"

using namespace Microsoft::VisualStudio::CppUnitTestFramework;
using namespace UnitOfNumber;

namespace UnitTest
{
	TEST_CLASS(UnitOfNumberUnitTest)
	{
	public:
		
		template<class T>
		constexpr auto CAST(T&& v) { return static_cast<NumeralValueType>(v); }

		TEST_METHOD(NumeralTestMethod_Constructor)
		{
			{
				Numeral test;
				Assert::AreEqual(CAST(0), CAST(test));
			}

			{
				Numeral test = 1;
				Assert::AreEqual(CAST(1), CAST(test));
			}
		}

		TEST_METHOD(NumeralTestMethod_Assignment)
		{
			Numeral test;
			test = 2;
			Assert::AreEqual(CAST(2), CAST(test));
		}

		TEST_METHOD(NumeralTestMethod_UnaryNegationPlus)
		{
			Numeral test = 3;
			Assert::AreEqual(CAST(+3), CAST(+test));
			Assert::AreEqual(CAST(-3), CAST(-test));
		}

		TEST_METHOD(NumeralTestMethod_Arithmetic)
		{
			Numeral test, lhs = 2, rhs = 3;

			test = lhs + rhs;
			Assert::AreEqual(CAST(5), CAST(test));
			Assert::AreEqual(CAST(10), test + 5);

			test = lhs - rhs;
			Assert::AreEqual(CAST(-1), CAST(test));
			Assert::AreEqual(CAST(4), test + 5);

			test = lhs * rhs;
			Assert::AreEqual(CAST(6), CAST(test));
			Assert::AreEqual(CAST(11), test + 5);

			test = lhs / rhs;
			Assert::AreEqual(CAST(0), CAST(test));
			Assert::AreEqual(CAST(5), test + 5);

			test = lhs % rhs;
			Assert::AreEqual(CAST(2), CAST(test));
			Assert::AreEqual(CAST(7), test + 5);
		}
	};
}
