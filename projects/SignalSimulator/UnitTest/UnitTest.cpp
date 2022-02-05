#include <gtest/gtest.h>
#include <time.h>
#include <windows.h>
#include <chrono>
#include "UnitOfNumber.hpp"
#include "ScientificPostulates.hpp"

class UnitOfNumberUnitTest : public ::testing::Test
{
protected:
	struct ProcessingTimeMeasurement
	{
		//C/C++で処理時間の計測を行う時，time.h で定義されている clock()関数が良く利用されます．
		//ただ，clock()関数の分解能は10[ms]程度ですので，短い処理の計測には向きません．
		struct LowPrecision
		{
			template<class T> static auto measurement(T&& Func)
			{
				clock_t start = clock();

				Func();

				clock_t end = clock();

				const double time = static_cast<double>(end - start) / CLOCKS_PER_SEC * 1000.0;

				return time;
			}
		};

		//clock関数を利用して短い処理の時間を計測したい場合，例えば下記のように繰り返し処理にして，
		//後で掛かった時間を繰り返し回数で割るような方法を考える必要があります．
		struct LowPrecisionImprovement
		{
			template<class T>static auto measurement(T&& Func)
			{
				clock_t start = clock();

				const int loop = 100;
				for (int i = 0; i < loop; i++)
				{
					Func();
				}

				clock_t end = clock();

				const double time = static_cast<double>(end - start) / CLOCKS_PER_SEC * 1000.0 / loop;

				return time;
			}
		};

		//<chrono>で定義されているクラスで，1[ms]程度の分解能で時間計測が可能です．
		//C++11をコンパイルできる環境があれば利用できるため，クロスプラットフォームを考える場合は有用だと思います．
		struct MediumPrecision
		{
			template<class T>static auto measurement(T&& Func)
			{
				using namespace std;
				chrono::system_clock::time_point start, end;

				start = chrono::system_clock::now();

				Func();

				end = chrono::system_clock::now();

				double time = static_cast<double>(chrono::duration_cast<chrono::microseconds>(end - start).count() / 1000.0);

				return time;
			}
		};

		//windows.hで定義されている関数で，1[ms]以下の細かい分解能で時間計測が可能です．
		//clock関数に比べると使用法がやや複雑ですが，精度は高いためwindows環境であれば採用を検討しても良いと思います．
		struct HighPrecision
		{
			template<class T>static auto measurement(T&& Func)
			{
				// QueryPerformanceCounter関数の1秒当たりのカウント数を取得する
				LARGE_INTEGER freq;
				QueryPerformanceFrequency(&freq);

				LARGE_INTEGER start, end;

				QueryPerformanceCounter(&start);

				Func();

				QueryPerformanceCounter(&end);

				double time = static_cast<double>(end.QuadPart - start.QuadPart) * 1000.0 / freq.QuadPart;

				return time;
			}
		};
	};
	using ProcessingTimeMeasurementFunc = ProcessingTimeMeasurement::HighPrecision;
};

class NumeralUnitTest : public UnitOfNumberUnitTest
{
protected:
	using NumeralValueType = intmax_t;
	//using NumeralValueType = long double;
	using Numeral = UnitOfNumber::Numeral<NumeralValueType>;

	template<class T>
	constexpr auto CAST(T&& v) { return static_cast<NumeralValueType>(v); }

	template<class T>
	void WriteMessageForTime(T time)
	{
		char out[256];
#pragma warning(suppress : 4996)
		sprintf(out, "time %lf[ms]\n", time);
		std::cout << out << std::endl;// デバッグ時のログ(出力欄)に出力
	}

	const double PARAM = 0.1;
};

TEST_F(NumeralUnitTest, TestMethod_Constructor)
{
	//引数なし
	{
		Numeral test;
		EXPECT_NEAR(CAST(0), CAST(test), PARAM);
	}
	
	//プリミティブ型参照左辺値
	{
		NumeralValueType test1 = 1;
		Numeral test2 = test1;
		EXPECT_NEAR(CAST(1), CAST(test1), PARAM);
		EXPECT_NEAR(CAST(1), CAST(test2), PARAM);
	}
	
	//プリミティブ型右辺値
	{
		Numeral test = NumeralValueType(2);
		EXPECT_NEAR(CAST(2), CAST(test), PARAM);
	}
	
	//Numeral型参照左辺値
	{
		Numeral test1 = 3;
		Numeral test2 = test1;
		EXPECT_NEAR(CAST(3), CAST(test1), PARAM);
		EXPECT_NEAR(CAST(3), CAST(test2), PARAM);
	}
	
	//Numeral型右辺値
	{
		Numeral test = Numeral(4);
		EXPECT_NEAR(CAST(4), CAST(test), PARAM);
	}
}

TEST_F(NumeralUnitTest, TestMethod_Assignment)
{
	//プリミティブ型参照左辺値
	{
		NumeralValueType test1 = 1;
		Numeral test2;
		test2 = test1;
		EXPECT_NEAR(CAST(1), CAST(test1), PARAM);
		EXPECT_NEAR(CAST(1), CAST(test2), PARAM);
	}

	//プリミティブ型右辺値
	{
		Numeral test;
		test = NumeralValueType(2);
		EXPECT_NEAR(CAST(2), CAST(test), PARAM);
	}

	//Numeral型参照左辺値
	{
		Numeral test1 = 3;
		Numeral test2;
		test2 = test1;
		EXPECT_NEAR(CAST(3), CAST(test1), PARAM);
		EXPECT_NEAR(CAST(3), CAST(test2), PARAM);
	}

	//Numeral型右辺値
	{
		Numeral test;
		test = Numeral(4);
		EXPECT_NEAR(CAST(4), CAST(test), PARAM);
	}
}

TEST_F(NumeralUnitTest, TestMethod_UnaryNegationPlus)
{
	Numeral test1 = 3, test2 = +test1, test3 = -test1;
	EXPECT_NEAR(CAST(+3), CAST(+test1), PARAM);
	EXPECT_NEAR(CAST(-3), CAST(-test1), PARAM);
	EXPECT_NEAR(CAST(+3), CAST(+test2), PARAM);
	EXPECT_NEAR(CAST(-3), CAST(-test2), PARAM);
	EXPECT_NEAR(CAST(-3), CAST(+test3), PARAM);
	EXPECT_NEAR(CAST(+3), CAST(-test3), PARAM);
}

TEST_F(NumeralUnitTest, TestMethod_Arithmetic)
{
	Numeral lhs = 2;
	Numeral rhs1 = 3;
	NumeralValueType rhs2 = 5;

	EXPECT_NEAR(CAST(5 ), CAST(lhs + rhs1), PARAM);
	EXPECT_NEAR(CAST(7 ), CAST(lhs + rhs2), PARAM);
	EXPECT_NEAR(CAST(9 ), CAST(lhs + Numeral(7)), PARAM);
	EXPECT_NEAR(CAST(11), CAST(lhs + NumeralValueType(9)), PARAM);

	EXPECT_NEAR(CAST(-1), CAST(lhs - rhs1), PARAM);
	EXPECT_NEAR(CAST(-3), CAST(lhs - rhs2), PARAM);
	EXPECT_NEAR(CAST(-5), CAST(lhs - Numeral(7)), PARAM);
	EXPECT_NEAR(CAST(-7), CAST(lhs - NumeralValueType(9)), PARAM);

	EXPECT_NEAR(CAST(6 ), CAST(lhs * rhs1), PARAM);
	EXPECT_NEAR(CAST(10), CAST(lhs * rhs2), PARAM);
	EXPECT_NEAR(CAST(14), CAST(lhs * Numeral(7)), PARAM);
	EXPECT_NEAR(CAST(18), CAST(lhs * NumeralValueType(9)), PARAM);

	if constexpr (std::is_integral<NumeralValueType>::value)
	{
		EXPECT_NEAR(CAST(0 ), CAST(lhs / rhs1), PARAM);
		EXPECT_NEAR(CAST(0 ), CAST(lhs / rhs2), PARAM);
		EXPECT_NEAR(CAST(0 ), CAST(lhs / Numeral(7)), PARAM);
		EXPECT_NEAR(CAST(0 ), CAST(lhs / NumeralValueType(9)), PARAM);	
	}
	else if constexpr (std::is_floating_point<NumeralValueType>::value)
	{
		EXPECT_NEAR(CAST(0.666667), CAST(lhs / rhs1), PARAM);
		EXPECT_NEAR(CAST(0.4     ), CAST(lhs / rhs2), PARAM);
		EXPECT_NEAR(CAST(0.285714), CAST(lhs / Numeral(7)), PARAM);
		EXPECT_NEAR(CAST(0.222222), CAST(lhs / NumeralValueType(9)), PARAM);
	}

	if constexpr (std::is_integral<NumeralValueType>::value)
	{
		EXPECT_NEAR(CAST(2 ), CAST(lhs % rhs1), PARAM);
		EXPECT_NEAR(CAST(2 ), CAST(lhs % rhs2), PARAM);
		EXPECT_NEAR(CAST(2 ), CAST(lhs % Numeral(7)), PARAM);
		EXPECT_NEAR(CAST(2 ), CAST(lhs % NumeralValueType(9)), PARAM);
	}
}

TEST_F(NumeralUnitTest, TestMethod_CompoundAssignment)
{
	Numeral lhs = 2;
	Numeral rhs1 = 3;
	NumeralValueType rhs2 = 5;

	lhs = 2;					EXPECT_NEAR(CAST(2   ), CAST(lhs), PARAM);
	lhs += rhs1;				EXPECT_NEAR(CAST(5   ), CAST(lhs), PARAM);
	lhs += rhs2;				EXPECT_NEAR(CAST(10  ), CAST(lhs), PARAM);
	lhs += Numeral(7);			EXPECT_NEAR(CAST(17  ), CAST(lhs), PARAM);
	lhs += NumeralValueType(9);	EXPECT_NEAR(CAST(26  ), CAST(lhs), PARAM);

	lhs = 2;					EXPECT_NEAR(CAST(2   ), CAST(lhs), PARAM);
	lhs -= rhs1;				EXPECT_NEAR(CAST(-1  ), CAST(lhs), PARAM);
	lhs -= rhs2;				EXPECT_NEAR(CAST(-6  ), CAST(lhs), PARAM);
	lhs -= Numeral(7);			EXPECT_NEAR(CAST(-13 ), CAST(lhs), PARAM);
	lhs -= NumeralValueType(9);	EXPECT_NEAR(CAST(-22 ), CAST(lhs), PARAM);

	lhs = 2;					EXPECT_NEAR(CAST(2   ), CAST(lhs), PARAM);
	lhs *= rhs1;				EXPECT_NEAR(CAST(6   ), CAST(lhs), PARAM);
	lhs *= rhs2;				EXPECT_NEAR(CAST(30  ), CAST(lhs), PARAM);
	lhs *= Numeral(7);			EXPECT_NEAR(CAST(210 ), CAST(lhs), PARAM);
	lhs *= NumeralValueType(9);	EXPECT_NEAR(CAST(1890), CAST(lhs), PARAM);

	if constexpr (std::is_integral<NumeralValueType>::value)
	{
		lhs = 2;					EXPECT_NEAR(CAST(2   ), CAST(lhs), PARAM);
		lhs /= rhs1;				EXPECT_NEAR(CAST(0   ), CAST(lhs), PARAM);
		lhs /= rhs2;				EXPECT_NEAR(CAST(0   ), CAST(lhs), PARAM);
		lhs /= Numeral(7);			EXPECT_NEAR(CAST(0   ), CAST(lhs), PARAM);
		lhs /= NumeralValueType(9);	EXPECT_NEAR(CAST(0   ), CAST(lhs), PARAM);
	}
	else if constexpr (std::is_floating_point<NumeralValueType>::value)
	{
		lhs = 2;					EXPECT_NEAR(CAST(2       ), CAST(lhs), PARAM);
		lhs /= rhs1;				EXPECT_NEAR(CAST(0.666667), CAST(lhs), PARAM);
		lhs /= rhs2;				EXPECT_NEAR(CAST(0.133333), CAST(lhs), PARAM);
		lhs /= Numeral(7);			EXPECT_NEAR(CAST(0       ), CAST(lhs), PARAM);
		lhs /= NumeralValueType(9);	EXPECT_NEAR(CAST(0       ), CAST(lhs), PARAM);
	}

	if constexpr (std::is_integral<NumeralValueType>::value)
	{
		lhs = 2;					EXPECT_NEAR(CAST(2   ), CAST(lhs), PARAM);
		lhs %= rhs1;				EXPECT_NEAR(CAST(2   ), CAST(lhs), PARAM);
		lhs %= rhs2;				EXPECT_NEAR(CAST(2   ), CAST(lhs), PARAM);
		lhs %= Numeral(7);			EXPECT_NEAR(CAST(2   ), CAST(lhs), PARAM);
		lhs %= NumeralValueType(9);	EXPECT_NEAR(CAST(2   ), CAST(lhs), PARAM);
	}
}

TEST_F(NumeralUnitTest, TestMethod_PostfixIncrementorDecrement)
{
	Numeral test = 0;
	EXPECT_NEAR(CAST(0 ), CAST(test++), PARAM);
	EXPECT_NEAR(CAST(1 ), CAST(test--), PARAM);
	EXPECT_NEAR(CAST(0 ), CAST(test--), PARAM);
	EXPECT_NEAR(CAST(-1), CAST(test--), PARAM);
	EXPECT_NEAR(CAST(-2), CAST(test++), PARAM);
	EXPECT_NEAR(CAST(-1), CAST(test++), PARAM);
	EXPECT_NEAR(CAST(0 ), CAST(test++), PARAM);
	EXPECT_NEAR(CAST(1 ), CAST(test++), PARAM);
}

TEST_F(NumeralUnitTest, TestMethod_PrefixIncrementorDecrement)
{
	Numeral test = 0;
	EXPECT_NEAR(CAST(1 ), CAST(++test), PARAM);
	EXPECT_NEAR(CAST(0 ), CAST(--test), PARAM);
	EXPECT_NEAR(CAST(-1), CAST(--test), PARAM);
	EXPECT_NEAR(CAST(-2), CAST(--test), PARAM);
	EXPECT_NEAR(CAST(-1), CAST(++test), PARAM);
	EXPECT_NEAR(CAST(0 ), CAST(++test), PARAM);
	EXPECT_NEAR(CAST(1 ), CAST(++test), PARAM);
	EXPECT_NEAR(CAST(2 ), CAST(++test), PARAM);
}

TEST_F(NumeralUnitTest, TestMethod_LogicalNot)
{
	Numeral test;
	test =  0; ASSERT_TRUE(!test);
	test =  1; ASSERT_FALSE(!test);
	test = -1; ASSERT_FALSE(!test);
}

TEST_F(NumeralUnitTest, TestMethod_Compare)
{
	Numeral lhs = 2;
	Numeral rhs1 = 3;
	NumeralValueType rhs2 = 5;
	Numeral rhs3 = lhs;
	
	ASSERT_FALSE(lhs == rhs1);
	ASSERT_FALSE(lhs == rhs2);
	ASSERT_TRUE(lhs  == rhs3);
	ASSERT_FALSE(lhs == Numeral(7));
	ASSERT_FALSE(lhs == NumeralValueType(9));
	
	ASSERT_TRUE(lhs  != rhs1);
	ASSERT_TRUE(lhs  != rhs2);
	ASSERT_FALSE(lhs != rhs3);
	ASSERT_TRUE(lhs  != Numeral(7));
	ASSERT_TRUE(lhs  != NumeralValueType(9));
	
	ASSERT_TRUE(lhs  <= rhs1);
	ASSERT_TRUE(lhs  <= rhs2);
	ASSERT_TRUE(lhs  <= rhs3);
	ASSERT_TRUE(lhs  <= Numeral(7));
	ASSERT_TRUE(lhs  <= NumeralValueType(9));
	
	ASSERT_TRUE(lhs  <  rhs1);
	ASSERT_TRUE(lhs  <  rhs2);
	ASSERT_FALSE(lhs <  rhs3);
	ASSERT_TRUE(lhs  <  Numeral(7));
	ASSERT_TRUE(lhs  <  NumeralValueType(9));
	
	ASSERT_FALSE(lhs >= rhs1);
	ASSERT_FALSE(lhs >= rhs2);
	ASSERT_TRUE(lhs  >= rhs3);
	ASSERT_FALSE(lhs >= Numeral(7));
	ASSERT_FALSE(lhs >= NumeralValueType(9));
	
	ASSERT_FALSE(lhs >  rhs1);
	ASSERT_FALSE(lhs >  rhs2);
	ASSERT_FALSE(lhs >  rhs3);
	ASSERT_FALSE(lhs >  Numeral(7));
	ASSERT_FALSE(lhs >  NumeralValueType(9));
}

TEST_F(NumeralUnitTest, TestMethod_ProcessingTimeMeasurement)
{
	constexpr NumeralValueType LENGTH = 1000 * 1000;
	{
		NumeralValueType sum = 0;
		auto func = [&]()
		{
			for (sum = 0; sum < LENGTH; sum++);
		};
		std::cout << "【プリミティブ型】" << std::endl;
		std::cout << "ループ1000000回実行" << std::endl;
		const auto time = ProcessingTimeMeasurementFunc::measurement(func);
		std::cout << "合計:"; WriteMessageForTime(time);
		std::cout << "平均:"; WriteMessageForTime(time / LENGTH);
		EXPECT_NEAR(CAST(LENGTH), CAST(sum), PARAM);
	}
	{
		Numeral sum = 0;
		auto func = [&]()
		{
			for (sum = 0; sum < LENGTH; sum++);
		};
		std::cout << "【Numeral型】" << std::endl;
		std::cout << "ループ1000000回実行" << std::endl;
		const auto time = ProcessingTimeMeasurementFunc::measurement(func);
		std::cout << "合計:"; WriteMessageForTime(time);
		std::cout << "平均:"; WriteMessageForTime(time / LENGTH);
		EXPECT_NEAR(CAST(LENGTH), CAST(sum), PARAM);
	}
}

class SIPrefixUnitTest : public UnitOfNumberUnitTest
{
protected:
	//using SIPrefixType = intmax_t;
	using SIPrefixType = long double;
	using SIPrefix = UnitOfNumber::SIPrefix<SIPrefixType>;

	template<class T>
	constexpr auto CAST(T&& v) { return static_cast<SIPrefixType>(v); }

	template<class T>
	void WriteMessageForTime(T time)
	{
		char out[256];
#pragma warning(suppress : 4996)
		sprintf(out, "time %lf[ms]\n", time);
		std::cout << out << std::endl;// デバッグ時のログ(出力欄)に出力
	}

	const double PARAM = 0.1;
};

TEST_F(SIPrefixUnitTest, TestMethod_Constructor)
{
	//引数なし
	{
		SIPrefix test;
		EXPECT_NEAR(CAST(0), CAST(test), PARAM);
	}

	//プリミティブ型参照左辺値
	{
		SIPrefixType test1 = 1;
		SIPrefix test2 = test1;
		EXPECT_NEAR(CAST(1), CAST(test1), PARAM);
		EXPECT_NEAR(CAST(1), CAST(test2), PARAM);
	}

	//プリミティブ型右辺値
	{
		SIPrefix test = SIPrefixType(2);
		EXPECT_NEAR(CAST(2), CAST(test), PARAM);
	}

	//Numeral型参照左辺値
	{
		SIPrefix test1 = 3;
		SIPrefix test2 = test1;
		EXPECT_NEAR(CAST(3), CAST(test1), PARAM);
		EXPECT_NEAR(CAST(3), CAST(test2), PARAM);
	}

	//Numeral型右辺値
	{
		SIPrefix test = SIPrefix(4);
		EXPECT_NEAR(CAST(4), CAST(test), PARAM);
	}
}

TEST_F(SIPrefixUnitTest, TestMethod_Assignment)
{
	//プリミティブ型参照左辺値
	{
		SIPrefixType test1 = 1;
		SIPrefix test2;
		test2 = test1;
		EXPECT_NEAR(CAST(1), CAST(test1), PARAM);
		EXPECT_NEAR(CAST(1), CAST(test2), PARAM);
	}

	//プリミティブ型右辺値
	{
		SIPrefix test;
		test = SIPrefixType(2);
		EXPECT_NEAR(CAST(2), CAST(test), PARAM);
	}

	//Numeral型参照左辺値
	{
		SIPrefix test1 = 3;
		SIPrefix test2;
		test2 = test1;
		EXPECT_NEAR(CAST(3), CAST(test1), PARAM);
		EXPECT_NEAR(CAST(3), CAST(test2), PARAM);
	}

	//Numeral型右辺値
	{
		SIPrefix test;
		test = SIPrefix(4);
		EXPECT_NEAR(CAST(4), CAST(test), PARAM);
	}
}

TEST_F(SIPrefixUnitTest, TestMethod_UnaryNegationPlus)
{
	SIPrefix test1 = 3, test2 = +test1, test3 = -test1;
	EXPECT_NEAR(CAST(+3), CAST(+test1), PARAM);
	EXPECT_NEAR(CAST(-3), CAST(-test1), PARAM);
	EXPECT_NEAR(CAST(+3), CAST(+test2), PARAM);
	EXPECT_NEAR(CAST(-3), CAST(-test2), PARAM);
	EXPECT_NEAR(CAST(-3), CAST(+test3), PARAM);
	EXPECT_NEAR(CAST(+3), CAST(-test3), PARAM);
}

TEST_F(SIPrefixUnitTest, TestMethod_Arithmetic)
{
	SIPrefix lhs = 2;
	SIPrefix rhs1 = 3;
	SIPrefixType rhs2 = 5;

	EXPECT_NEAR(CAST(5 ), CAST(lhs + rhs1), PARAM);
	EXPECT_NEAR(CAST(7 ), CAST(lhs + rhs2), PARAM);
	EXPECT_NEAR(CAST(9 ), CAST(lhs + SIPrefix(7)), PARAM);
	EXPECT_NEAR(CAST(11), CAST(lhs + SIPrefixType(9)), PARAM);

	EXPECT_NEAR(CAST(-1), CAST(lhs - rhs1), PARAM);
	EXPECT_NEAR(CAST(-3), CAST(lhs - rhs2), PARAM);
	EXPECT_NEAR(CAST(-5), CAST(lhs - SIPrefix(7)), PARAM);
	EXPECT_NEAR(CAST(-7), CAST(lhs - SIPrefixType(9)), PARAM);

	EXPECT_NEAR(CAST(6 ), CAST(lhs * rhs1), PARAM);
	EXPECT_NEAR(CAST(10), CAST(lhs * rhs2), PARAM);
	EXPECT_NEAR(CAST(14), CAST(lhs * SIPrefix(7)), PARAM);
	EXPECT_NEAR(CAST(18), CAST(lhs * SIPrefixType(9)), PARAM);

	if constexpr (std::is_integral<SIPrefixType>::value)
	{
		EXPECT_NEAR(CAST(0 ), CAST(lhs / rhs1), PARAM);
		EXPECT_NEAR(CAST(0 ), CAST(lhs / rhs2), PARAM);
		EXPECT_NEAR(CAST(0 ), CAST(lhs / SIPrefix(7)), PARAM);
		EXPECT_NEAR(CAST(0 ), CAST(lhs / SIPrefixType(9)), PARAM);
	}
	else if constexpr (std::is_floating_point<SIPrefixType>::value)
	{
		EXPECT_NEAR(CAST(0.666667), CAST(lhs / rhs1), PARAM);
		EXPECT_NEAR(CAST(0.4     ), CAST(lhs / rhs2), PARAM);
		EXPECT_NEAR(CAST(0.285714), CAST(lhs / SIPrefix(7)), PARAM);
		EXPECT_NEAR(CAST(0.222222), CAST(lhs / SIPrefixType(9)), PARAM);
	}

	if constexpr (std::is_integral<SIPrefixType>::value)
	{
		EXPECT_NEAR(CAST(2 ), CAST(lhs % rhs1), PARAM);
		EXPECT_NEAR(CAST(2 ), CAST(lhs % rhs2), PARAM);
		EXPECT_NEAR(CAST(2 ), CAST(lhs % SIPrefix(7)), PARAM);
		EXPECT_NEAR(CAST(2 ), CAST(lhs % SIPrefixType(9)), PARAM);
	}
}

TEST_F(SIPrefixUnitTest, TestMethod_CompoundAssignment)
{
	SIPrefix lhs = 2;
	SIPrefix rhs1 = 3;
	SIPrefixType rhs2 = 5;

	lhs = 2;					EXPECT_NEAR(CAST(2   ), CAST(lhs), PARAM);
	lhs += rhs1;				EXPECT_NEAR(CAST(5   ), CAST(lhs), PARAM);
	lhs += rhs2;				EXPECT_NEAR(CAST(10  ), CAST(lhs), PARAM);
	lhs += SIPrefix(7);			EXPECT_NEAR(CAST(17  ), CAST(lhs), PARAM);
	lhs += SIPrefixType(9);	EXPECT_NEAR(CAST(26  ), CAST(lhs), PARAM);

	lhs = 2;					EXPECT_NEAR(CAST(2   ), CAST(lhs), PARAM);
	lhs -= rhs1;				EXPECT_NEAR(CAST(-1  ), CAST(lhs), PARAM);
	lhs -= rhs2;				EXPECT_NEAR(CAST(-6  ), CAST(lhs), PARAM);
	lhs -= SIPrefix(7);			EXPECT_NEAR(CAST(-13 ), CAST(lhs), PARAM);
	lhs -= SIPrefixType(9);	EXPECT_NEAR(CAST(-22 ), CAST(lhs), PARAM);

	lhs = 2;					EXPECT_NEAR(CAST(2   ), CAST(lhs), PARAM);
	lhs *= rhs1;				EXPECT_NEAR(CAST(6   ), CAST(lhs), PARAM);
	lhs *= rhs2;				EXPECT_NEAR(CAST(30  ), CAST(lhs), PARAM);
	lhs *= SIPrefix(7);			EXPECT_NEAR(CAST(210 ), CAST(lhs), PARAM);
	lhs *= SIPrefixType(9);	EXPECT_NEAR(CAST(1890), CAST(lhs), PARAM);

	if constexpr (std::is_integral<SIPrefixType>::value)
	{
		lhs = 2;					EXPECT_NEAR(CAST(2   ), CAST(lhs), PARAM);
		lhs /= rhs1;				EXPECT_NEAR(CAST(0   ), CAST(lhs), PARAM);
		lhs /= rhs2;				EXPECT_NEAR(CAST(0   ), CAST(lhs), PARAM);
		lhs /= SIPrefix(7);			EXPECT_NEAR(CAST(0   ), CAST(lhs), PARAM);
		lhs /= SIPrefixType(9);	EXPECT_NEAR(CAST(0   ), CAST(lhs), PARAM);
	}
	else if constexpr (std::is_floating_point<SIPrefixType>::value)
	{
		lhs = 2;					EXPECT_NEAR(CAST(2       ), CAST(lhs), PARAM);
		lhs /= rhs1;				EXPECT_NEAR(CAST(0.666667), CAST(lhs), PARAM);
		lhs /= rhs2;				EXPECT_NEAR(CAST(0.133333), CAST(lhs), PARAM);
		lhs /= SIPrefix(7);			EXPECT_NEAR(CAST(0       ), CAST(lhs), PARAM);
		lhs /= SIPrefixType(9);	EXPECT_NEAR(CAST(0       ), CAST(lhs), PARAM);
	}

	if constexpr (std::is_integral<SIPrefixType>::value)
	{
		lhs = 2;					EXPECT_NEAR(CAST(2   ), CAST(lhs), PARAM);
		lhs %= rhs1;				EXPECT_NEAR(CAST(2   ), CAST(lhs), PARAM);
		lhs %= rhs2;				EXPECT_NEAR(CAST(2   ), CAST(lhs), PARAM);
		lhs %= SIPrefix(7);			EXPECT_NEAR(CAST(2   ), CAST(lhs), PARAM);
		lhs %= SIPrefixType(9);	EXPECT_NEAR(CAST(2   ), CAST(lhs), PARAM);
	}
}

TEST_F(SIPrefixUnitTest, TestMethod_PostfixIncrementorDecrement)
{
	SIPrefix test = 0;
	EXPECT_NEAR(CAST(0 ), CAST(test++), PARAM);
	EXPECT_NEAR(CAST(1 ), CAST(test--), PARAM);
	EXPECT_NEAR(CAST(0 ), CAST(test--), PARAM);
	EXPECT_NEAR(CAST(-1), CAST(test--), PARAM);
	EXPECT_NEAR(CAST(-2), CAST(test++), PARAM);
	EXPECT_NEAR(CAST(-1), CAST(test++), PARAM);
	EXPECT_NEAR(CAST(0 ), CAST(test++), PARAM);
	EXPECT_NEAR(CAST(1 ), CAST(test++), PARAM);
}

TEST_F(SIPrefixUnitTest, TestMethod_PrefixIncrementorDecrement)
{
	SIPrefix test = 0;
	EXPECT_NEAR(CAST(1 ), CAST(++test), PARAM);
	EXPECT_NEAR(CAST(0 ), CAST(--test), PARAM);
	EXPECT_NEAR(CAST(-1), CAST(--test), PARAM);
	EXPECT_NEAR(CAST(-2), CAST(--test), PARAM);
	EXPECT_NEAR(CAST(-1), CAST(++test), PARAM);
	EXPECT_NEAR(CAST(0 ), CAST(++test), PARAM);
	EXPECT_NEAR(CAST(1 ), CAST(++test), PARAM);
	EXPECT_NEAR(CAST(2 ), CAST(++test), PARAM);
}

TEST_F(SIPrefixUnitTest, TestMethod_LogicalNot)
{
	SIPrefix test;
	test =  0; ASSERT_TRUE(!test);
	test =  1; ASSERT_FALSE(!test);
	test = -1; ASSERT_FALSE(!test);
}

TEST_F(SIPrefixUnitTest, TestMethod_Compare)
{
	SIPrefix lhs = 2;
	SIPrefix rhs1 = 3;
	SIPrefixType rhs2 = 5;
	SIPrefix rhs3 = lhs;

	ASSERT_FALSE(lhs == rhs1);
	ASSERT_FALSE(lhs == rhs2);
	ASSERT_TRUE(lhs  == rhs3);
	ASSERT_FALSE(lhs == SIPrefix(7));
	ASSERT_FALSE(lhs == SIPrefixType(9));

	ASSERT_TRUE(lhs  != rhs1);
	ASSERT_TRUE(lhs  != rhs2);
	ASSERT_FALSE(lhs != rhs3);
	ASSERT_TRUE(lhs  != SIPrefix(7));
	ASSERT_TRUE(lhs  != SIPrefixType(9));

	ASSERT_TRUE(lhs  <= rhs1);
	ASSERT_TRUE(lhs  <= rhs2);
	ASSERT_TRUE(lhs  <= rhs3);
	ASSERT_TRUE(lhs  <= SIPrefix(7));
	ASSERT_TRUE(lhs  <= SIPrefixType(9));

	ASSERT_TRUE(lhs  <  rhs1);
	ASSERT_TRUE(lhs  <  rhs2);
	ASSERT_FALSE(lhs <  rhs3);
	ASSERT_TRUE(lhs  < SIPrefix(7));
	ASSERT_TRUE(lhs  < SIPrefixType(9));

	ASSERT_FALSE(lhs >= rhs1);
	ASSERT_FALSE(lhs >= rhs2);
	ASSERT_TRUE(lhs  >= rhs3);
	ASSERT_FALSE(lhs >= SIPrefix(7));
	ASSERT_FALSE(lhs >= SIPrefixType(9));

	ASSERT_FALSE(lhs >  rhs1);
	ASSERT_FALSE(lhs >  rhs2);
	ASSERT_FALSE(lhs >  rhs3);
	ASSERT_FALSE(lhs > SIPrefix(7));
	ASSERT_FALSE(lhs > SIPrefixType(9));
}

TEST_F(SIPrefixUnitTest, TestMethod_ConvertFromOriginalToSIPrefixUnit)
{
	//{
	//	SIPrefix test = 1;
	//
	//	EXPECT_NEAR(CAST(0.000000000000000000000000000001), CAST(test.Q), PARAM);
	//	EXPECT_NEAR(CAST(0.000000000000000000000000001),	 CAST(test.R), PARAM);
	//	EXPECT_NEAR(CAST(0.000000000000000000000001),		 CAST(test.Y), PARAM);
	//	EXPECT_NEAR(CAST(0.000000000000000000001),			 CAST(test.Z), PARAM);
	//	EXPECT_NEAR(CAST(0.000000000000000001),			 CAST(test.E), PARAM);
	//	EXPECT_NEAR(CAST(0.000000000000001),				 CAST(test.P), PARAM);
	//	EXPECT_NEAR(CAST(0.000000000001),					 CAST(test.T), PARAM);
	//	EXPECT_NEAR(CAST(0.000000001),						 CAST(test.G), PARAM);
	//	EXPECT_NEAR(CAST(0.000001),						 CAST(test.M), PARAM);
	//	EXPECT_NEAR(CAST(0.001),							 CAST(test.k), PARAM);
	//	EXPECT_NEAR(CAST(0.01),							 CAST(test.h), PARAM);
	//	EXPECT_NEAR(CAST(0.1),								 CAST(test.da), PARAM);
	//	EXPECT_NEAR(CAST(1),								 CAST(test.base), PARAM);
	//	EXPECT_NEAR(CAST(10),								 CAST(test.d), PARAM);
	//	EXPECT_NEAR(CAST(100),								 CAST(test.c), PARAM);
	//	EXPECT_NEAR(CAST(1000),							 CAST(test.m), PARAM);
	//	EXPECT_NEAR(CAST(1000000),							 CAST(test.u), PARAM);
	//	EXPECT_NEAR(CAST(1000000000),						 CAST(test.n), PARAM);
	//	EXPECT_NEAR(CAST(1000000000000),					 CAST(test.p), PARAM);
	//	EXPECT_NEAR(CAST(1000000000000000),				 CAST(test.f), PARAM);
	//	EXPECT_NEAR(CAST(1000000000000000000),				 CAST(test.a), PARAM);
	//	//start このテストはソフト的に不可
	//	//EXPECT_NEAR(CAST(1000000000000000000000),			CAST(test.z), PARAM);
	//	//EXPECT_NEAR(CAST(1000000000000000000000000),			CAST(test.y), PARAM);
	//	//EXPECT_NEAR(CAST(1000000000000000000000000000),		CAST(test.r), PARAM);
	//	//EXPECT_NEAR(CAST(1000000000000000000000000000000),	CAST(test.q), PARAM);
	//	//end
	//}
	{
		SIPrefix test = 1;
	
		EXPECT_NEAR(CAST(1),								 CAST(test.base), PARAM);
		EXPECT_NEAR(CAST(10),								 CAST(test.d), PARAM);
		EXPECT_NEAR(CAST(100),								 CAST(test.c), PARAM);
		EXPECT_NEAR(CAST(1000),							 CAST(test.m), PARAM);
		EXPECT_NEAR(CAST(1000000),							 CAST(test.u), PARAM);
		EXPECT_NEAR(CAST(1000000000),						 CAST(test.n), PARAM);
		EXPECT_NEAR(CAST(1000000000000),					 CAST(test.p), PARAM);
		EXPECT_NEAR(CAST(1000000000000000),				 CAST(test.f), PARAM);
		EXPECT_NEAR(CAST(1000000000000000000),				 CAST(test.a), PARAM);
	}
	{
		SIPrefix test = 1000000000000000000;
		
		EXPECT_NEAR(CAST(1),								 CAST(test.E), PARAM);
		EXPECT_NEAR(CAST(1000),							 CAST(test.P), PARAM);
		EXPECT_NEAR(CAST(1000000),							 CAST(test.T), PARAM);
		EXPECT_NEAR(CAST(1000000000),						 CAST(test.G), PARAM);
		EXPECT_NEAR(CAST(1000000000000),					 CAST(test.M), PARAM);
		EXPECT_NEAR(CAST(1000000000000000),				 CAST(test.k), PARAM);
		EXPECT_NEAR(CAST(10000000000000000),				 CAST(test.h), PARAM);
		EXPECT_NEAR(CAST(100000000000000000),				 CAST(test.da), PARAM);
		EXPECT_NEAR(CAST(1000000000000000000),				 CAST(test.base), PARAM);
	}
}

TEST_F(SIPrefixUnitTest, TestMethod_ConvertFromSIPrefixUnitToOriginal)
{
	SIPrefix test;
	test.E = 1;
	EXPECT_NEAR(CAST(1000000000000000000), CAST(test.base), PARAM);
}

TEST_F(SIPrefixUnitTest, TestMethod_ConvertFromSIPrefixUnitToOriginal2)
{
	SIPrefix test;

	test.k = 1;
	test.k = test.k + test.k;
	EXPECT_NEAR(CAST(2000), CAST(test), PARAM);
	test.k = 1;
	test = test.k + test.k;
	EXPECT_NEAR(CAST(2), CAST(test), PARAM);
}

TEST_F(SIPrefixUnitTest, TestMethod_ProcessingTimeMeasurement)
{
	constexpr SIPrefixType LENGTH = 1000 * 1000;
	{
		SIPrefixType sum = 0;
		auto func = [&]()
		{
			for (sum = 0; sum < LENGTH; sum++);
		};
		std::cout << "【プリミティブ型】" << std::endl;
		std::cout << "ループ1000000回実行" << std::endl;
		const auto time = ProcessingTimeMeasurementFunc::measurement(func);
		std::cout << "合計:"; WriteMessageForTime(time);
		std::cout << "平均:"; WriteMessageForTime(time / LENGTH);
		EXPECT_NEAR(CAST(LENGTH), CAST(sum), PARAM);
	}
	{
		SIPrefix sum = 0;
		auto func = [&]()
		{
			for (sum = 0; sum < LENGTH; sum++);
		};
		std::cout << "【Numeral型】" << std::endl;
		std::cout << "ループ1000000回実行" << std::endl;
		const auto time = ProcessingTimeMeasurementFunc::measurement(func);
		std::cout << "合計:"; WriteMessageForTime(time);
		std::cout << "平均:"; WriteMessageForTime(time / LENGTH);
		EXPECT_NEAR(CAST(LENGTH), CAST(sum), PARAM);
	}
}

class ScientificPostulatesUnitTest : public UnitOfNumberUnitTest
{
protected:
	template<class T>
	constexpr auto CAST(T&& v) { return static_cast<UnitOfNumber::ScientificPostulates::Type>(v); }

	template<class T>
	void WriteMessageForValue(T value)
	{
		char out[256];
#pragma warning(suppress : 4996)
		sprintf(out, "value %lf", value);
		std::cout << out << std::endl;// デバッグ時のログ(出力欄)に出力
	}
};

TEST_F(ScientificPostulatesUnitTest, TestMethod_Display)
{
	WriteMessageForValue(CAST(UnitOfNumber::SP.C  ));
	WriteMessageForValue(CAST(UnitOfNumber::SP.Co ));
	WriteMessageForValue(CAST(UnitOfNumber::SP.g  ));
	WriteMessageForValue(CAST(UnitOfNumber::SP.G  ));
	WriteMessageForValue(CAST(UnitOfNumber::SP.me ));
	WriteMessageForValue(CAST(UnitOfNumber::SP.mp ));
	WriteMessageForValue(CAST(UnitOfNumber::SP.mn ));
	WriteMessageForValue(CAST(UnitOfNumber::SP.ou ));
	WriteMessageForValue(CAST(UnitOfNumber::SP.e0 ));
	WriteMessageForValue(CAST(UnitOfNumber::SP.u0 ));
	WriteMessageForValue(CAST(UnitOfNumber::SP.h  ));
	WriteMessageForValue(CAST(UnitOfNumber::SP.e  ));
	WriteMessageForValue(CAST(UnitOfNumber::SP.Rif));
	WriteMessageForValue(CAST(UnitOfNumber::SP.NA ));
	WriteMessageForValue(CAST(UnitOfNumber::SP.L  ));
	WriteMessageForValue(CAST(UnitOfNumber::SP.Vm ));
	WriteMessageForValue(CAST(UnitOfNumber::SP.F  ));
	WriteMessageForValue(CAST(UnitOfNumber::SP.R  ));
	WriteMessageForValue(CAST(UnitOfNumber::SP.k  ));
	WriteMessageForValue(CAST(UnitOfNumber::SP.t  ));
	WriteMessageForValue(CAST(UnitOfNumber::SP.KJ ));
	WriteMessageForValue(CAST(UnitOfNumber::SP.atm));
}
