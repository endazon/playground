#include "pch.h"
#include "CppUnitTest.h"
#include "UnitOfNumber.h"

using namespace Microsoft::VisualStudio::CppUnitTestFramework;

namespace UnitTest
{
	struct ProcessingTimeMeasurement {

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

	TEST_CLASS(NumeralUnitTest)
	{
		using NumeralValueType = intmax_t;
		//using NumeralValueType = long double;
		using Numeral = UnitOfNumber::Numeral<NumeralValueType>;

	private:
		template<class T>
		constexpr auto CAST(T&& v) { return static_cast<NumeralValueType>(v); }

		template<class T>
		void WriteMessageForTime(T time)
		{
			char out[256];
#pragma warning(suppress : 4996)
			sprintf(out, "time %lf[ms]\n", time);
			Logger::WriteMessage(out);// デバッグ時のログ(出力欄)に出力
		}

		const double PARAM = 0.1;

	public:
		TEST_METHOD(TestMethod_Constructor)
		{
			//引数なし
			{
				Numeral test;
				Assert::AreEqual(CAST(0), CAST(test), PARAM);
			}

			//プリミティブ型参照左辺値
			{
				NumeralValueType test1 = 1;
				Numeral test2 = test1;
				Assert::AreEqual(CAST(1), CAST(test1), PARAM);
				Assert::AreEqual(CAST(1), CAST(test2), PARAM);
			}

			//プリミティブ型右辺値
			{
				Numeral test = NumeralValueType(2);
				Assert::AreEqual(CAST(2), CAST(test), PARAM);
			}

			//Numeral型参照左辺値
			{
				Numeral test1 = 3;
				Numeral test2 = test1;
				Assert::AreEqual(CAST(3), CAST(test1), PARAM);
				Assert::AreEqual(CAST(3), CAST(test2), PARAM);
			}

			//Numeral型右辺値
			{
				Numeral test = Numeral(4);
				Assert::AreEqual(CAST(4), CAST(test), PARAM);
			}
		}

		TEST_METHOD(TestMethod_Assignment)
		{	
			//プリミティブ型参照左辺値
			{
				NumeralValueType test1 = 1;
				Numeral test2;
				test2 = test1;
				Assert::AreEqual(CAST(1), CAST(test1), PARAM);
				Assert::AreEqual(CAST(1), CAST(test2), PARAM);
			}

			//プリミティブ型右辺値
			{
				Numeral test;
				test = NumeralValueType(2);
				Assert::AreEqual(CAST(2), CAST(test), PARAM);
			}

			//Numeral型参照左辺値
			{
				Numeral test1 = 3;
				Numeral test2;
				test2 = test1;
				Assert::AreEqual(CAST(3), CAST(test1), PARAM);
				Assert::AreEqual(CAST(3), CAST(test2), PARAM);
			}

			//Numeral型右辺値
			{
				Numeral test;
				test = Numeral(4);
				Assert::AreEqual(CAST(4), CAST(test), PARAM);
			}
		}

		TEST_METHOD(TestMethod_UnaryNegationPlus)
		{
			Numeral test1 = 3, test2 = +test1, test3 = -test1;
			Assert::AreEqual(CAST(+3), CAST(+test1), PARAM);
			Assert::AreEqual(CAST(-3), CAST(-test1), PARAM);
			Assert::AreEqual(CAST(+3), CAST(+test2), PARAM);
			Assert::AreEqual(CAST(-3), CAST(-test2), PARAM);
			Assert::AreEqual(CAST(-3), CAST(+test3), PARAM);
			Assert::AreEqual(CAST(+3), CAST(-test3), PARAM);
		}

		TEST_METHOD(TestMethod_Arithmetic)
		{
			Numeral lhs = 2;
			Numeral rhs1 = 3;
			NumeralValueType rhs2 = 5;

			Assert::AreEqual(CAST(5 ), CAST(lhs + rhs1), PARAM);
			Assert::AreEqual(CAST(7 ), CAST(lhs + rhs2), PARAM);
			Assert::AreEqual(CAST(9 ), CAST(lhs + Numeral(7)), PARAM);
			Assert::AreEqual(CAST(11), CAST(lhs + NumeralValueType(9)), PARAM);

			Assert::AreEqual(CAST(-1), CAST(lhs - rhs1), PARAM);
			Assert::AreEqual(CAST(-3), CAST(lhs - rhs2), PARAM);
			Assert::AreEqual(CAST(-5), CAST(lhs - Numeral(7)), PARAM);
			Assert::AreEqual(CAST(-7), CAST(lhs - NumeralValueType(9)), PARAM);

			Assert::AreEqual(CAST(6 ), CAST(lhs * rhs1), PARAM);
			Assert::AreEqual(CAST(10), CAST(lhs * rhs2), PARAM);
			Assert::AreEqual(CAST(14), CAST(lhs * Numeral(7)), PARAM);
			Assert::AreEqual(CAST(18), CAST(lhs * NumeralValueType(9)), PARAM);

			if constexpr (std::is_integral<NumeralValueType>::value)
			{
				Assert::AreEqual(CAST(0 ), CAST(lhs / rhs1), PARAM);
				Assert::AreEqual(CAST(0 ), CAST(lhs / rhs2), PARAM);
				Assert::AreEqual(CAST(0 ), CAST(lhs / Numeral(7)), PARAM);
				Assert::AreEqual(CAST(0 ), CAST(lhs / NumeralValueType(9)), PARAM);	
			}
			else if constexpr (std::is_floating_point<NumeralValueType>::value)
			{
				Assert::AreEqual(CAST(0.666667), CAST(lhs / rhs1), PARAM);
				Assert::AreEqual(CAST(0.4     ), CAST(lhs / rhs2), PARAM);
				Assert::AreEqual(CAST(0.285714), CAST(lhs / Numeral(7)), PARAM);
				Assert::AreEqual(CAST(0.222222), CAST(lhs / NumeralValueType(9)), PARAM);
			}

			if constexpr (std::is_integral<NumeralValueType>::value)
			{
				Assert::AreEqual(CAST(2 ), CAST(lhs % rhs1), PARAM);
				Assert::AreEqual(CAST(2 ), CAST(lhs % rhs2), PARAM);
				Assert::AreEqual(CAST(2 ), CAST(lhs % Numeral(7)), PARAM);
				Assert::AreEqual(CAST(2 ), CAST(lhs % NumeralValueType(9)), PARAM);
			}
		}

		TEST_METHOD(TestMethod_CompoundAssignment)
		{
			Numeral lhs = 2;
			Numeral rhs1 = 3;
			NumeralValueType rhs2 = 5;

			lhs = 2;					Assert::AreEqual(CAST(2   ), CAST(lhs), PARAM);
			lhs += rhs1;				Assert::AreEqual(CAST(5   ), CAST(lhs), PARAM);
			lhs += rhs2;				Assert::AreEqual(CAST(10  ), CAST(lhs), PARAM);
			lhs += Numeral(7);			Assert::AreEqual(CAST(17  ), CAST(lhs), PARAM);
			lhs += NumeralValueType(9);	Assert::AreEqual(CAST(26  ), CAST(lhs), PARAM);

			lhs = 2;					Assert::AreEqual(CAST(2   ), CAST(lhs), PARAM);
			lhs -= rhs1;				Assert::AreEqual(CAST(-1  ), CAST(lhs), PARAM);
			lhs -= rhs2;				Assert::AreEqual(CAST(-6  ), CAST(lhs), PARAM);
			lhs -= Numeral(7);			Assert::AreEqual(CAST(-13 ), CAST(lhs), PARAM);
			lhs -= NumeralValueType(9);	Assert::AreEqual(CAST(-22 ), CAST(lhs), PARAM);

			lhs = 2;					Assert::AreEqual(CAST(2   ), CAST(lhs), PARAM);
			lhs *= rhs1;				Assert::AreEqual(CAST(6   ), CAST(lhs), PARAM);
			lhs *= rhs2;				Assert::AreEqual(CAST(30  ), CAST(lhs), PARAM);
			lhs *= Numeral(7);			Assert::AreEqual(CAST(210 ), CAST(lhs), PARAM);
			lhs *= NumeralValueType(9);	Assert::AreEqual(CAST(1890), CAST(lhs), PARAM);

			if constexpr (std::is_integral<NumeralValueType>::value)
			{
				lhs = 2;					Assert::AreEqual(CAST(2   ), CAST(lhs), PARAM);
				lhs /= rhs1;				Assert::AreEqual(CAST(0   ), CAST(lhs), PARAM);
				lhs /= rhs2;				Assert::AreEqual(CAST(0   ), CAST(lhs), PARAM);
				lhs /= Numeral(7);			Assert::AreEqual(CAST(0   ), CAST(lhs), PARAM);
				lhs /= NumeralValueType(9);	Assert::AreEqual(CAST(0   ), CAST(lhs), PARAM);
			}
			else if constexpr (std::is_floating_point<NumeralValueType>::value)
			{
				lhs = 2;					Assert::AreEqual(CAST(2       ), CAST(lhs), PARAM);
				lhs /= rhs1;				Assert::AreEqual(CAST(0.666667), CAST(lhs), PARAM);
				lhs /= rhs2;				Assert::AreEqual(CAST(0.133333), CAST(lhs), PARAM);
				lhs /= Numeral(7);			Assert::AreEqual(CAST(0       ), CAST(lhs), PARAM);
				lhs /= NumeralValueType(9);	Assert::AreEqual(CAST(0       ), CAST(lhs), PARAM);
			}

			if constexpr (std::is_integral<NumeralValueType>::value)
			{
				lhs = 2;					Assert::AreEqual(CAST(2   ), CAST(lhs), PARAM);
				lhs %= rhs1;				Assert::AreEqual(CAST(2   ), CAST(lhs), PARAM);
				lhs %= rhs2;				Assert::AreEqual(CAST(2   ), CAST(lhs), PARAM);
				lhs %= Numeral(7);			Assert::AreEqual(CAST(2   ), CAST(lhs), PARAM);
				lhs %= NumeralValueType(9);	Assert::AreEqual(CAST(2   ), CAST(lhs), PARAM);
			}
		}

		TEST_METHOD(TestMethod_PostfixIncrementorDecrement)
		{
			Numeral test = 0;
			Assert::AreEqual(CAST(0 ), CAST(test++), PARAM);
			Assert::AreEqual(CAST(1 ), CAST(test--), PARAM);
			Assert::AreEqual(CAST(0 ), CAST(test--), PARAM);
			Assert::AreEqual(CAST(-1), CAST(test--), PARAM);
			Assert::AreEqual(CAST(-2), CAST(test++), PARAM);
			Assert::AreEqual(CAST(-1), CAST(test++), PARAM);
			Assert::AreEqual(CAST(0 ), CAST(test++), PARAM);
			Assert::AreEqual(CAST(1 ), CAST(test++), PARAM);
		}

		TEST_METHOD(TestMethod_PrefixIncrementorDecrement)
		{
			Numeral test = 0;
			Assert::AreEqual(CAST(1 ), CAST(++test), PARAM);
			Assert::AreEqual(CAST(0 ), CAST(--test), PARAM);
			Assert::AreEqual(CAST(-1), CAST(--test), PARAM);
			Assert::AreEqual(CAST(-2), CAST(--test), PARAM);
			Assert::AreEqual(CAST(-1), CAST(++test), PARAM);
			Assert::AreEqual(CAST(0 ), CAST(++test), PARAM);
			Assert::AreEqual(CAST(1 ), CAST(++test), PARAM);
			Assert::AreEqual(CAST(2 ), CAST(++test), PARAM);
		}

		TEST_METHOD(TestMethod_LogicalNot)
		{
			Numeral test;
			test =  0; Assert::IsTrue(!test);
			test =  1; Assert::IsFalse(!test);
			test = -1; Assert::IsFalse(!test);
		}

		TEST_METHOD(TestMethod_Compare)
		{
			Numeral lhs = 2;
			Numeral rhs1 = 3;
			NumeralValueType rhs2 = 5;
			Numeral rhs3 = lhs;

			Assert::IsFalse(lhs == rhs1);
			Assert::IsFalse(lhs == rhs2);
			Assert::IsTrue(lhs  == rhs3);
			Assert::IsFalse(lhs == Numeral(7));
			Assert::IsFalse(lhs == NumeralValueType(9));

			Assert::IsTrue(lhs  != rhs1);
			Assert::IsTrue(lhs  != rhs2);
			Assert::IsFalse(lhs != rhs3);
			Assert::IsTrue(lhs  != Numeral(7));
			Assert::IsTrue(lhs  != NumeralValueType(9));

			Assert::IsTrue(lhs  <= rhs1);
			Assert::IsTrue(lhs  <= rhs2);
			Assert::IsTrue(lhs  <= rhs3);
			Assert::IsTrue(lhs  <= Numeral(7));
			Assert::IsTrue(lhs  <= NumeralValueType(9));

			Assert::IsTrue(lhs  <  rhs1);
			Assert::IsTrue(lhs  <  rhs2);
			Assert::IsFalse(lhs <  rhs3);
			Assert::IsTrue(lhs  <  Numeral(7));
			Assert::IsTrue(lhs  <  NumeralValueType(9));

			Assert::IsFalse(lhs >= rhs1);
			Assert::IsFalse(lhs >= rhs2);
			Assert::IsTrue(lhs  >= rhs3);
			Assert::IsFalse(lhs >= Numeral(7));
			Assert::IsFalse(lhs >= NumeralValueType(9));

			Assert::IsFalse(lhs >  rhs1);
			Assert::IsFalse(lhs >  rhs2);
			Assert::IsFalse(lhs >  rhs3);
			Assert::IsFalse(lhs >  Numeral(7));
			Assert::IsFalse(lhs >  NumeralValueType(9));
		}

		TEST_METHOD(TestMethod_ProcessingTimeMeasurement)
		{
			constexpr NumeralValueType LENGTH = 1000 * 1000;
			{
				NumeralValueType sum = 0;
				auto func = [&]()
				{
					for (sum = 0; sum < LENGTH; sum++);
				};
				Logger::WriteMessage("【プリミティブ型】\n");
				Logger::WriteMessage("ループ1000000回実行\n");
				const auto time = ProcessingTimeMeasurementFunc::measurement(func);
				Logger::WriteMessage("合計:"); WriteMessageForTime(time);
				Logger::WriteMessage("平均:"); WriteMessageForTime(time / LENGTH);
				Assert::AreEqual(CAST(LENGTH), CAST(sum), PARAM);
			}
			{
				Numeral sum = 0;
				auto func = [&]()
				{
					for (sum = 0; sum < LENGTH; sum++);
				};
				Logger::WriteMessage("【Numeral型】\n");
				Logger::WriteMessage("ループ1000000回実行\n");
				const auto time = ProcessingTimeMeasurementFunc::measurement(func);
				Logger::WriteMessage("合計:"); WriteMessageForTime(time);
				Logger::WriteMessage("平均:"); WriteMessageForTime(time / LENGTH);
				Assert::AreEqual(CAST(LENGTH), CAST(sum), PARAM);
			}
		}
	};

	TEST_CLASS(SIPrefixUnitTest)
	{
		//using SIPrefixType = intmax_t;
		using SIPrefixType = long double;
		using SIPrefix = UnitOfNumber::SIPrefix<SIPrefixType>;

	private:
		template<class T>
		constexpr auto CAST(T&& v) { return static_cast<SIPrefixType>(v); }

		template<class T>
		void WriteMessageForTime(T time)
		{
			char out[256];
#pragma warning(suppress : 4996)
			sprintf(out, "time %lf[ms]\n", time);
			Logger::WriteMessage(out);// デバッグ時のログ(出力欄)に出力
		}

		const double PARAM = 0.1;

	public:
		TEST_METHOD(TestMethod_Constructor)
		{
			//引数なし
			{
				SIPrefix test;
				Assert::AreEqual(CAST(0), CAST(test), PARAM);
			}

			//プリミティブ型参照左辺値
			{
				SIPrefixType test1 = 1;
				SIPrefix test2 = test1;
				Assert::AreEqual(CAST(1), CAST(test1), PARAM);
				Assert::AreEqual(CAST(1), CAST(test2), PARAM);
			}

			//プリミティブ型右辺値
			{
				SIPrefix test = SIPrefixType(2);
				Assert::AreEqual(CAST(2), CAST(test), PARAM);
			}

			//Numeral型参照左辺値
			{
				SIPrefix test1 = 3;
				SIPrefix test2 = test1;
				Assert::AreEqual(CAST(3), CAST(test1), PARAM);
				Assert::AreEqual(CAST(3), CAST(test2), PARAM);
			}

			//Numeral型右辺値
			{
				SIPrefix test = SIPrefix(4);
				Assert::AreEqual(CAST(4), CAST(test), PARAM);
			}
		}

		TEST_METHOD(TestMethod_Assignment)
		{	
			//プリミティブ型参照左辺値
			{
				SIPrefixType test1 = 1;
				SIPrefix test2;
				test2 = test1;
				Assert::AreEqual(CAST(1), CAST(test1), PARAM);
				Assert::AreEqual(CAST(1), CAST(test2), PARAM);
			}

			//プリミティブ型右辺値
			{
				SIPrefix test;
				test = SIPrefixType(2);
				Assert::AreEqual(CAST(2), CAST(test), PARAM);
			}

			//Numeral型参照左辺値
			{
				SIPrefix test1 = 3;
				SIPrefix test2;
				test2 = test1;
				Assert::AreEqual(CAST(3), CAST(test1), PARAM);
				Assert::AreEqual(CAST(3), CAST(test2), PARAM);
			}

			//Numeral型右辺値
			{
				SIPrefix test;
				test = SIPrefix(4);
				Assert::AreEqual(CAST(4), CAST(test), PARAM);
			}
		}

		TEST_METHOD(TestMethod_UnaryNegationPlus)
		{
			SIPrefix test1 = 3, test2 = +test1, test3 = -test1;
			Assert::AreEqual(CAST(+3), CAST(+test1), PARAM);
			Assert::AreEqual(CAST(-3), CAST(-test1), PARAM);
			Assert::AreEqual(CAST(+3), CAST(+test2), PARAM);
			Assert::AreEqual(CAST(-3), CAST(-test2), PARAM);
			Assert::AreEqual(CAST(-3), CAST(+test3), PARAM);
			Assert::AreEqual(CAST(+3), CAST(-test3), PARAM);
		}

		TEST_METHOD(TestMethod_Arithmetic)
		{
			SIPrefix lhs = 2;
			SIPrefix rhs1 = 3;
			SIPrefixType rhs2 = 5;

			Assert::AreEqual(CAST(5 ), CAST(lhs + rhs1), PARAM);
			Assert::AreEqual(CAST(7 ), CAST(lhs + rhs2), PARAM);
			Assert::AreEqual(CAST(9 ), CAST(lhs + SIPrefix(7)), PARAM);
			Assert::AreEqual(CAST(11), CAST(lhs + SIPrefixType(9)), PARAM);

			Assert::AreEqual(CAST(-1), CAST(lhs - rhs1), PARAM);
			Assert::AreEqual(CAST(-3), CAST(lhs - rhs2), PARAM);
			Assert::AreEqual(CAST(-5), CAST(lhs - SIPrefix(7)), PARAM);
			Assert::AreEqual(CAST(-7), CAST(lhs - SIPrefixType(9)), PARAM);

			Assert::AreEqual(CAST(6 ), CAST(lhs * rhs1), PARAM);
			Assert::AreEqual(CAST(10), CAST(lhs * rhs2), PARAM);
			Assert::AreEqual(CAST(14), CAST(lhs * SIPrefix(7)), PARAM);
			Assert::AreEqual(CAST(18), CAST(lhs * SIPrefixType(9)), PARAM);

			if constexpr (std::is_integral<SIPrefixType>::value)
			{
				Assert::AreEqual(CAST(0 ), CAST(lhs / rhs1), PARAM);
				Assert::AreEqual(CAST(0 ), CAST(lhs / rhs2), PARAM);
				Assert::AreEqual(CAST(0 ), CAST(lhs / SIPrefix(7)), PARAM);
				Assert::AreEqual(CAST(0 ), CAST(lhs / SIPrefixType(9)), PARAM);
			}
			else if constexpr (std::is_floating_point<SIPrefixType>::value)
			{
				Assert::AreEqual(CAST(0.666667), CAST(lhs / rhs1), PARAM);
				Assert::AreEqual(CAST(0.4     ), CAST(lhs / rhs2), PARAM);
				Assert::AreEqual(CAST(0.285714), CAST(lhs / SIPrefix(7)), PARAM);
				Assert::AreEqual(CAST(0.222222), CAST(lhs / SIPrefixType(9)), PARAM);
			}

			if constexpr (std::is_integral<SIPrefixType>::value)
			{
				Assert::AreEqual(CAST(2 ), CAST(lhs % rhs1), PARAM);
				Assert::AreEqual(CAST(2 ), CAST(lhs % rhs2), PARAM);
				Assert::AreEqual(CAST(2 ), CAST(lhs % SIPrefix(7)), PARAM);
				Assert::AreEqual(CAST(2 ), CAST(lhs % SIPrefixType(9)), PARAM);
			}
		}

		TEST_METHOD(TestMethod_CompoundAssignment)
		{
			SIPrefix lhs = 2;
			SIPrefix rhs1 = 3;
			SIPrefixType rhs2 = 5;

			lhs = 2;					Assert::AreEqual(CAST(2   ), CAST(lhs), PARAM);
			lhs += rhs1;				Assert::AreEqual(CAST(5   ), CAST(lhs), PARAM);
			lhs += rhs2;				Assert::AreEqual(CAST(10  ), CAST(lhs), PARAM);
			lhs += SIPrefix(7);			Assert::AreEqual(CAST(17  ), CAST(lhs), PARAM);
			lhs += SIPrefixType(9);	Assert::AreEqual(CAST(26  ), CAST(lhs), PARAM);

			lhs = 2;					Assert::AreEqual(CAST(2   ), CAST(lhs), PARAM);
			lhs -= rhs1;				Assert::AreEqual(CAST(-1  ), CAST(lhs), PARAM);
			lhs -= rhs2;				Assert::AreEqual(CAST(-6  ), CAST(lhs), PARAM);
			lhs -= SIPrefix(7);			Assert::AreEqual(CAST(-13 ), CAST(lhs), PARAM);
			lhs -= SIPrefixType(9);	Assert::AreEqual(CAST(-22 ), CAST(lhs), PARAM);

			lhs = 2;					Assert::AreEqual(CAST(2   ), CAST(lhs), PARAM);
			lhs *= rhs1;				Assert::AreEqual(CAST(6   ), CAST(lhs), PARAM);
			lhs *= rhs2;				Assert::AreEqual(CAST(30  ), CAST(lhs), PARAM);
			lhs *= SIPrefix(7);			Assert::AreEqual(CAST(210 ), CAST(lhs), PARAM);
			lhs *= SIPrefixType(9);	Assert::AreEqual(CAST(1890), CAST(lhs), PARAM);

			if constexpr (std::is_integral<SIPrefixType>::value)
			{
				lhs = 2;					Assert::AreEqual(CAST(2   ), CAST(lhs), PARAM);
				lhs /= rhs1;				Assert::AreEqual(CAST(0   ), CAST(lhs), PARAM);
				lhs /= rhs2;				Assert::AreEqual(CAST(0   ), CAST(lhs), PARAM);
				lhs /= SIPrefix(7);			Assert::AreEqual(CAST(0   ), CAST(lhs), PARAM);
				lhs /= SIPrefixType(9);	Assert::AreEqual(CAST(0   ), CAST(lhs), PARAM);
			}
			else if constexpr (std::is_floating_point<SIPrefixType>::value)
			{
				lhs = 2;					Assert::AreEqual(CAST(2       ), CAST(lhs), PARAM);
				lhs /= rhs1;				Assert::AreEqual(CAST(0.666667), CAST(lhs), PARAM);
				lhs /= rhs2;				Assert::AreEqual(CAST(0.133333), CAST(lhs), PARAM);
				lhs /= SIPrefix(7);			Assert::AreEqual(CAST(0       ), CAST(lhs), PARAM);
				lhs /= SIPrefixType(9);	Assert::AreEqual(CAST(0       ), CAST(lhs), PARAM);
			}

			if constexpr (std::is_integral<SIPrefixType>::value)
			{
				lhs = 2;					Assert::AreEqual(CAST(2   ), CAST(lhs), PARAM);
				lhs %= rhs1;				Assert::AreEqual(CAST(2   ), CAST(lhs), PARAM);
				lhs %= rhs2;				Assert::AreEqual(CAST(2   ), CAST(lhs), PARAM);
				lhs %= SIPrefix(7);			Assert::AreEqual(CAST(2   ), CAST(lhs), PARAM);
				lhs %= SIPrefixType(9);	Assert::AreEqual(CAST(2   ), CAST(lhs), PARAM);
			}
		}

		TEST_METHOD(TestMethod_PostfixIncrementorDecrement)
		{
			SIPrefix test = 0;
			Assert::AreEqual(CAST(0 ), CAST(test++), PARAM);
			Assert::AreEqual(CAST(1 ), CAST(test--), PARAM);
			Assert::AreEqual(CAST(0 ), CAST(test--), PARAM);
			Assert::AreEqual(CAST(-1), CAST(test--), PARAM);
			Assert::AreEqual(CAST(-2), CAST(test++), PARAM);
			Assert::AreEqual(CAST(-1), CAST(test++), PARAM);
			Assert::AreEqual(CAST(0 ), CAST(test++), PARAM);
			Assert::AreEqual(CAST(1 ), CAST(test++), PARAM);
		}

		TEST_METHOD(TestMethod_PrefixIncrementorDecrement)
		{
			SIPrefix test = 0;
			Assert::AreEqual(CAST(1 ), CAST(++test), PARAM);
			Assert::AreEqual(CAST(0 ), CAST(--test), PARAM);
			Assert::AreEqual(CAST(-1), CAST(--test), PARAM);
			Assert::AreEqual(CAST(-2), CAST(--test), PARAM);
			Assert::AreEqual(CAST(-1), CAST(++test), PARAM);
			Assert::AreEqual(CAST(0 ), CAST(++test), PARAM);
			Assert::AreEqual(CAST(1 ), CAST(++test), PARAM);
			Assert::AreEqual(CAST(2 ), CAST(++test), PARAM);
		}

		TEST_METHOD(TestMethod_LogicalNot)
		{
			SIPrefix test;
			test =  0; Assert::IsTrue(!test);
			test =  1; Assert::IsFalse(!test);
			test = -1; Assert::IsFalse(!test);
		}

		TEST_METHOD(TestMethod_Compare)
		{
			SIPrefix lhs = 2;
			SIPrefix rhs1 = 3;
			SIPrefixType rhs2 = 5;
			SIPrefix rhs3 = lhs;

			Assert::IsFalse(lhs == rhs1);
			Assert::IsFalse(lhs == rhs2);
			Assert::IsTrue(lhs  == rhs3);
			Assert::IsFalse(lhs == SIPrefix(7));
			Assert::IsFalse(lhs == SIPrefixType(9));

			Assert::IsTrue(lhs  != rhs1);
			Assert::IsTrue(lhs  != rhs2);
			Assert::IsFalse(lhs != rhs3);
			Assert::IsTrue(lhs  != SIPrefix(7));
			Assert::IsTrue(lhs  != SIPrefixType(9));

			Assert::IsTrue(lhs  <= rhs1);
			Assert::IsTrue(lhs  <= rhs2);
			Assert::IsTrue(lhs  <= rhs3);
			Assert::IsTrue(lhs  <= SIPrefix(7));
			Assert::IsTrue(lhs  <= SIPrefixType(9));

			Assert::IsTrue(lhs  <  rhs1);
			Assert::IsTrue(lhs  <  rhs2);
			Assert::IsFalse(lhs <  rhs3);
			Assert::IsTrue(lhs  < SIPrefix(7));
			Assert::IsTrue(lhs  < SIPrefixType(9));

			Assert::IsFalse(lhs >= rhs1);
			Assert::IsFalse(lhs >= rhs2);
			Assert::IsTrue(lhs  >= rhs3);
			Assert::IsFalse(lhs >= SIPrefix(7));
			Assert::IsFalse(lhs >= SIPrefixType(9));

			Assert::IsFalse(lhs >  rhs1);
			Assert::IsFalse(lhs >  rhs2);
			Assert::IsFalse(lhs >  rhs3);
			Assert::IsFalse(lhs > SIPrefix(7));
			Assert::IsFalse(lhs > SIPrefixType(9));
		}

		TEST_METHOD(TestMethod_ConvertFromOriginalToSIPrefixUnit)
		{
			//{
			//	SIPrefix test = 1;
			//
			//	Assert::AreEqual(CAST(0.000000000000000000000000000001), CAST(test.Q), PARAM);
			//	Assert::AreEqual(CAST(0.000000000000000000000000001),	 CAST(test.R), PARAM);
			//	Assert::AreEqual(CAST(0.000000000000000000000001),		 CAST(test.Y), PARAM);
			//	Assert::AreEqual(CAST(0.000000000000000000001),			 CAST(test.Z), PARAM);
			//	Assert::AreEqual(CAST(0.000000000000000001),			 CAST(test.E), PARAM);
			//	Assert::AreEqual(CAST(0.000000000000001),				 CAST(test.P), PARAM);
			//	Assert::AreEqual(CAST(0.000000000001),					 CAST(test.T), PARAM);
			//	Assert::AreEqual(CAST(0.000000001),						 CAST(test.G), PARAM);
			//	Assert::AreEqual(CAST(0.000001),						 CAST(test.M), PARAM);
			//	Assert::AreEqual(CAST(0.001),							 CAST(test.k), PARAM);
			//	Assert::AreEqual(CAST(0.01),							 CAST(test.h), PARAM);
			//	Assert::AreEqual(CAST(0.1),								 CAST(test.da), PARAM);
			//	Assert::AreEqual(CAST(1),								 CAST(test.base), PARAM);
			//	Assert::AreEqual(CAST(10),								 CAST(test.d), PARAM);
			//	Assert::AreEqual(CAST(100),								 CAST(test.c), PARAM);
			//	Assert::AreEqual(CAST(1000),							 CAST(test.m), PARAM);
			//	Assert::AreEqual(CAST(1000000),							 CAST(test.u), PARAM);
			//	Assert::AreEqual(CAST(1000000000),						 CAST(test.n), PARAM);
			//	Assert::AreEqual(CAST(1000000000000),					 CAST(test.p), PARAM);
			//	Assert::AreEqual(CAST(1000000000000000),				 CAST(test.f), PARAM);
			//	Assert::AreEqual(CAST(1000000000000000000),				 CAST(test.a), PARAM);
			//	//start このテストはソフト的に不可
			//	//Assert::AreEqual(CAST(1000000000000000000000),			CAST(test.z), PARAM);
			//	//Assert::AreEqual(CAST(1000000000000000000000000),			CAST(test.y), PARAM);
			//	//Assert::AreEqual(CAST(1000000000000000000000000000),		CAST(test.r), PARAM);
			//	//Assert::AreEqual(CAST(1000000000000000000000000000000),	CAST(test.q), PARAM);
			//	//end
			//}
			{
				SIPrefix test = 1;

				Assert::AreEqual(CAST(1),								 CAST(test.base), PARAM);
				Assert::AreEqual(CAST(10),								 CAST(test.d), PARAM);
				Assert::AreEqual(CAST(100),								 CAST(test.c), PARAM);
				Assert::AreEqual(CAST(1000),							 CAST(test.m), PARAM);
				Assert::AreEqual(CAST(1000000),							 CAST(test.u), PARAM);
				Assert::AreEqual(CAST(1000000000),						 CAST(test.n), PARAM);
				Assert::AreEqual(CAST(1000000000000),					 CAST(test.p), PARAM);
				Assert::AreEqual(CAST(1000000000000000),				 CAST(test.f), PARAM);
				Assert::AreEqual(CAST(1000000000000000000),				 CAST(test.a), PARAM);
			}
			{
				SIPrefix test = 1000000000000000000;
				
				Assert::AreEqual(CAST(1),								 CAST(test.E), PARAM);
				Assert::AreEqual(CAST(1000),							 CAST(test.P), PARAM);
				Assert::AreEqual(CAST(1000000),							 CAST(test.T), PARAM);
				Assert::AreEqual(CAST(1000000000),						 CAST(test.G), PARAM);
				Assert::AreEqual(CAST(1000000000000),					 CAST(test.M), PARAM);
				Assert::AreEqual(CAST(1000000000000000),				 CAST(test.k), PARAM);
				Assert::AreEqual(CAST(10000000000000000),				 CAST(test.h), PARAM);
				Assert::AreEqual(CAST(100000000000000000),				 CAST(test.da), PARAM);
				Assert::AreEqual(CAST(1000000000000000000),				 CAST(test.base), PARAM);
			}
		}

		TEST_METHOD(TestMethod_ConvertFromSIPrefixUnitToOriginal)
		{
			SIPrefix test;
			test.E = 1;
			Assert::AreEqual(CAST(1000000000000000000), CAST(test.base), PARAM);
		}

		TEST_METHOD(TestMethod_ConvertFromSIPrefixUnitToOriginal2)
		{
			SIPrefix test;

			test.k = 1;
			test.k = test.k + test.k;
			Assert::AreEqual(CAST(2000), CAST(test), PARAM);
			test.k = 1;
			test = test.k + test.k;
			Assert::AreEqual(CAST(2), CAST(test), PARAM);
		}

		TEST_METHOD(TestMethod_ProcessingTimeMeasurement)
		{
			constexpr SIPrefixType LENGTH = 1000 * 1000;
			{
				SIPrefixType sum = 0;
				auto func = [&]()
				{
					for (sum = 0; sum < LENGTH; sum++);
				};
				Logger::WriteMessage("【プリミティブ型】\n");
				Logger::WriteMessage("ループ1000000回実行\n");
				const auto time = ProcessingTimeMeasurementFunc::measurement(func);
				Logger::WriteMessage("合計:"); WriteMessageForTime(time);
				Logger::WriteMessage("平均:"); WriteMessageForTime(time / LENGTH);
				Assert::AreEqual(CAST(LENGTH), CAST(sum), PARAM);
			}
			{
				SIPrefix sum = 0;
				auto func = [&]()
				{
					for (sum = 0; sum < LENGTH; sum++);
				};
				Logger::WriteMessage("【Numeral型】\n");
				Logger::WriteMessage("ループ1000000回実行\n");
				const auto time = ProcessingTimeMeasurementFunc::measurement(func);
				Logger::WriteMessage("合計:"); WriteMessageForTime(time);
				Logger::WriteMessage("平均:"); WriteMessageForTime(time / LENGTH);
				Assert::AreEqual(CAST(LENGTH), CAST(sum), PARAM);
			}
		}
	};
}
