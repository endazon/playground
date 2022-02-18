
#pragma once

#include <iostream>
#include <stdint.h>
#include <type_traits>
#include <cassert>
#include <time.h>
#include <chrono>
#include <windows.h>
#include <sstream>
#include <regex>

namespace GeneralPurposeTimer
{
	namespace Type
	{
		template<class __InheritanceType>
		class timeOperation : public __InheritanceType
		{
		public:
			using __InheritanceType::__InheritanceType;

			inline void print(__InheritanceType::rep count) { std::cout << count << this->units2Display() << std::endl; }
			inline void print()								{ print(this->count()); }
		};

		class nanosecondsOperation : public std::chrono::nanoseconds
		{
			using __InheritanceType = std::chrono::nanoseconds;

		public:
			using __InheritanceType::__InheritanceType;
			constexpr nanosecondsOperation(const std::chrono::nanoseconds& t) noexcept : std::chrono::nanoseconds(t) {}

			constexpr std::string units2Display() { return "[ns]"; }
		};
		class microsecondsOperation : public std::chrono::microseconds
		{
			using __InheritanceType = std::chrono::microseconds;

		public:
			using __InheritanceType::__InheritanceType;
			constexpr microsecondsOperation(const std::chrono::microseconds& t) noexcept : std::chrono::microseconds(t) {}

			constexpr std::string units2Display() { return "[us]"; }
		};
		class millisecondsOperation : public std::chrono::milliseconds
		{
			using __InheritanceType = std::chrono::milliseconds;

		public:
			using __InheritanceType::__InheritanceType;
			constexpr millisecondsOperation(const std::chrono::milliseconds& t) noexcept : std::chrono::milliseconds(t) {}

			constexpr std::string units2Display() { return "[ms]"; }
		};
		class secondsOperation : public std::chrono::seconds
		{
			using __InheritanceType = std::chrono::seconds;

		public:
			using __InheritanceType::__InheritanceType;
			constexpr secondsOperation(const std::chrono::seconds& t) noexcept : std::chrono::seconds(t) {}

			constexpr std::string units2Display() { return "[s]"; }
		};
		class minutesOperation : public std::chrono::minutes
		{
			using __InheritanceType = std::chrono::minutes;

		public:
			using __InheritanceType::__InheritanceType;
			constexpr minutesOperation(const std::chrono::minutes& t) noexcept : std::chrono::minutes(t) {}

			constexpr std::string units2Display() { return "[min]"; }
		};
		class hoursOperation : public std::chrono::hours
		{
			using __InheritanceType = std::chrono::hours;

		public:
			using __InheritanceType::__InheritanceType;
			constexpr hoursOperation(const std::chrono::hours& t) noexcept : std::chrono::hours(t) {}

			constexpr std::string units2Display() { return "[h]"; }
		};
		class daysOperation : public std::chrono::days
		{
			using __InheritanceType = std::chrono::days;

		public:
			using __InheritanceType::__InheritanceType;
			constexpr daysOperation(const std::chrono::days& t) noexcept : std::chrono::days(t) {}

			constexpr std::string units2Display() { return "[d]"; }
		};
		class weeksOperation : public std::chrono::weeks
		{
			using __InheritanceType = std::chrono::weeks;

		public:
			using __InheritanceType::__InheritanceType;
			constexpr weeksOperation(const std::chrono::weeks& t) noexcept : std::chrono::weeks(t) {}

			constexpr std::string units2Display() { return "[w]"; }
		};
		class monthsOperation : public std::chrono::months
		{
			using __InheritanceType = std::chrono::months;

		public:
			using __InheritanceType::__InheritanceType;
			constexpr monthsOperation(const std::chrono::months& t) noexcept : std::chrono::months(t) {}

			constexpr std::string units2Display() { return "[m]"; }
		};
		class yearsOperation : public std::chrono::years
		{
			using __InheritanceType = std::chrono::years;

		public:
			using __InheritanceType::__InheritanceType;
			constexpr yearsOperation(const std::chrono::years& t) noexcept : std::chrono::years(t) {}

			constexpr std::string units2Display() { return "[y]"; }
		};

		using nanoseconds	= timeOperation<nanosecondsOperation>;
		using microseconds	= timeOperation<microsecondsOperation>;
		using milliseconds	= timeOperation<millisecondsOperation>;
		using seconds		= timeOperation<secondsOperation>;
		using minutes		= timeOperation<minutesOperation>;
		using hours			= timeOperation<hoursOperation>;
		using days			= timeOperation<daysOperation>;
		using weeks			= timeOperation<weeksOperation>;
		using months		= timeOperation<monthsOperation>;
		using years			= timeOperation<yearsOperation>;
	}

	//参考にしたサイト
	//http://www.sanko-shoko.net/note.php?id=rnfd
	namespace Measurement
	{
		//C/C++で処理時間の計測を行う時，time.h で定義されている clock()関数が良く利用されます．
		//ただ，clock()関数の分解能は10[ms]程度ですので，短い処理の計測には向きません．
		class LowPrecision
		{
		public:
			using __CountType = Type::milliseconds;

			inline __CountType getTime()
			{
				return __CountType(clock());
			}
		};

		//<chrono>で定義されているクラスで，1[ms]程度の分解能で時間計測が可能です．
		//C++11をコンパイルできる環境があれば利用できるため，クロスプラットフォームを考える場合は有用だと思います．
		class MediumPrecision
		{
		public:
			using __CountType = Type::microseconds;

			inline __CountType getTime()
			{
				return __CountType(std::chrono::duration_cast<std::chrono::microseconds>(std::chrono::system_clock::now().time_since_epoch()));
			}
		};

		//windows.hで定義されている関数で，1[ms]以下の細かい分解能で時間計測が可能です．
		//clock関数に比べると使用法がやや複雑ですが，精度は高いためwindows環境であれば採用を検討しても良いと思います．
		class HighPrecision
		{
		private:
			size_t DigitAdjustment = 1;

		public:
			using __CountType = Type::nanoseconds;

			HighPrecision()
			{
				LARGE_INTEGER time;
				QueryPerformanceFrequency(&time);
				for (size_t i = 0; i < (static_cast<int>(log10(time.QuadPart)) + 1) % 3; i++) { DigitAdjustment *= 10; }
			}

			inline __CountType getTime()
			{
				LARGE_INTEGER time;
				QueryPerformanceCounter(&time);
				return __CountType(time.QuadPart * DigitAdjustment);
			}
		};

		template<class __PrecisionType>
		class MeasuringElapsedTime
		{
			static_assert(
				std::is_same<__PrecisionType, LowPrecision>::value    ||
				std::is_same<__PrecisionType, MediumPrecision>::value ||
				std::is_same<__PrecisionType, HighPrecision>::value   ,
				"type error"
				);

		private:
			using __CountType = __PrecisionType::__CountType;

			__PrecisionType timerAccessor;
			__CountType startTime;
			__CountType endTime;

		protected:
			inline void setStartTine(){ startTime = timerAccessor.getTime(); }
			inline void setEndTine()  { endTime   = timerAccessor.getTime(); }
			inline void print()
			{
				__CountType elapsedTime = endTime - startTime;
				elapsedTime.print();
			}

		public:
			inline MeasuringElapsedTime() noexcept  { setStartTine();          }
			inline ~MeasuringElapsedTime() noexcept { setEndTine();   print(); }
		};

		template<class __PrecisionType>
		class ElapsedTimeDetection
		{			
			static_assert(
				std::is_same<__PrecisionType, LowPrecision>::value    ||
				std::is_same<__PrecisionType, MediumPrecision>::value ||
				std::is_same<__PrecisionType, HighPrecision>::value   ,
				"type error"
				);

		private:
			using __CountType = __PrecisionType::__CountType;

			__PrecisionType timerAccessor;
			__CountType BasePoint;
			const __CountType THRESHOLD;

			__CountType getElapsedTime()
			{
				__CountType atPresent = timerAccessor.getTime();

				//オーバーフロー
				//if (atPresent < BasePoint) {
				//
				//}
				return atPresent - BasePoint;
			}

			inline void basePointUpdate()
			{
				BasePoint = timerAccessor.getTime();
			}

		public:
			template<class T> inline ElapsedTimeDetection(const T& time) noexcept : THRESHOLD(__CountType(time))
			{
				basePointUpdate();
			}
			template<class T> inline ElapsedTimeDetection(const T&& time) noexcept : ElapsedTimeDetection(time)
			{}

			bool isElapsed() 
			{
				if (THRESHOLD < getElapsedTime())
				{
					basePointUpdate();
					return true;
				}
				return false;
			}			
		};
	}

	//参考にしたサイト
	//https://memo.appri.me/programming/cpp-date-time
	//https://cpprefjp.github.io/reference/chrono/local_time.html
	namespace DateFormat
	{
		class UTC
		{

		private:
			const std::string_view TimeZone;

		public:
			UTC(std::string_view _tz = std::chrono::current_zone()->name()) :TimeZone(_tz) {}

			inline auto now()
			{
				// local_timeは、システム時間のエポックからの経過時間によって構築できる
				return std::chrono::local_time<std::chrono::system_clock::duration>(std::chrono::system_clock::now().time_since_epoch());
			}

			inline auto timezone()
			{
				return std::chrono::zoned_time(TimeZone, now());
			}

			inline auto format()
			{
				auto tz     = timezone();
				auto time   = std::chrono::duration_cast<std::chrono::milliseconds>(tz.get_local_time().time_since_epoch()).count();
				auto msec   = time % 1000;
				auto sec    = time / 1000;
				tm lt = {}; localtime_s(&lt, &sec);
				lt.tm_mon  += 1;
				lt.tm_year += 1900;

				char str[sizeof("[YYYY-MM-DDThh:mm:ss.fff XXX]")];
				assert(sprintf_s(str, "[%04d-%02d-%02dT%02d:%02d:%02d", lt.tm_year, lt.tm_mon, lt.tm_mday, lt.tm_hour, lt.tm_min, lt.tm_sec) == 20);
				assert(sprintf_s(str, "%s.%03I64d %s]", str, msec, tz.get_info().abbrev.c_str()) == 29);
				return std::string(str);
			}

			void print()
			{
				// 日時を出力
				std::cout << "[" << format() << "]" << std::endl;
			}
		};
	}
}