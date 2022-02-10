
#pragma once

#include <iostream>
#include <stdint.h>
#include <type_traits>
#include <cassert>
#include <time.h>
#include <chrono>
#include <windows.h>

namespace GeneralPurposeTimer
{
	enum class TimeUnits : uint8_t
	{
		nanoseconds,
		microseconds,
		milliseconds,
		seconds,
		minutes,
		hours,
		days,
		weeks,
		years,
		months,
	};

	namespace TimerAccessor
	{
		//C/C++で処理時間の計測を行う時，time.h で定義されている clock()関数が良く利用されます．
		//ただ，clock()関数の分解能は10[ms]程度ですので，短い処理の計測には向きません．
		class LowPrecision
		{
		public:
			using CountType = clock_t;

			inline CountType GetCount()
			{
				return clock();
			}

			inline void PrintForSCount(CountType cnt)
			{
				std::cout << cnt / (1000) << "[S]" << std::endl;
			}
			inline void PrintForMsCount(CountType cnt)
			{
				std::cout << cnt << "[ms]" << std::endl;
			}
			inline void PrintForUsCount(CountType cnt)
			{
				std::cout << cnt * (1000*1000) << "[us]" << std::endl;
			}
			inline void PrintForNsCount(CountType cnt)
			{
				std::cout << cnt * (1000*1000*1000) << "[ns]" << std::endl;
			}
		};

		//<chrono>で定義されているクラスで，1[ms]程度の分解能で時間計測が可能です．
		//C++11をコンパイルできる環境があれば利用できるため，クロスプラットフォームを考える場合は有用だと思います．
		class MediumPrecision
		{
		public:
			using CountType = std::chrono::nanoseconds::rep;

			inline CountType GetCount()
			{
				//return  std::chrono::system_clock::to_time_t(std::chrono::system_clock::now());
				return std::chrono::duration_cast<std::chrono::nanoseconds>(std::chrono::system_clock::now().time_since_epoch()).count();
			}

			inline void PrintForSCount(CountType cnt)
			{
				std::cout << cnt / (1000*1000*1000) << "[S]" << std::endl;
			}
			inline void PrintForMsCount(CountType cnt)
			{
				std::cout << cnt / (1000*1000) << "[ms]" << std::endl;
			}
			inline void PrintForUsCount(CountType cnt)
			{
				std::cout << cnt / (1000) << "[us]" << std::endl;
			}
			inline void PrintForNsCount(CountType cnt)
			{
				std::cout << cnt << "[ns]" << std::endl;
			}
		};

		//windows.hで定義されている関数で，1[ms]以下の細かい分解能で時間計測が可能です．
		//clock関数に比べると使用法がやや複雑ですが，精度は高いためwindows環境であれば採用を検討しても良いと思います．
		class HighPrecision
		{
		public:
			using CountType = LONGLONG;

			inline CountType GetCount()
			{
				LARGE_INTEGER time;
				QueryPerformanceCounter(&time);
				return time.QuadPart;
			}

			inline void PrintForSCount(CountType cnt)
			{
				std::cout << cnt / (10*1000*1000) << "[S]" << std::endl;
			}
			inline void PrintForMsCount(CountType cnt)
			{
				std::cout << cnt / (10*1000*1000) << "[ms]" << std::endl;
			}
			inline void PrintForUsCount(CountType cnt)
			{
				std::cout << cnt / (10*1000) << "[us]" << std::endl;
			}
			inline void PrintForNsCount(CountType cnt)
			{
				std::cout << cnt << "[ns]" << std::endl;
			}

			inline CountType GetCountsPerSecond()
			{
				LARGE_INTEGER time;
				QueryPerformanceFrequency(&time);
				return time.QuadPart;
			}
		};
	}

	template<class __TimerAccessorType, TimeUnits __Units = TimeUnits::seconds>
	class MeasuringElapsedTime
	{
	private:
		__TimerAccessorType timerAccessor;
		__TimerAccessorType::CountType startTime;
		__TimerAccessorType::CountType endTime;

	protected:
		inline void setStartTine(){ startTime = timerAccessor.GetCount(); }
		inline void setEndTine()  { endTime   = timerAccessor.GetCount(); }
		inline void Print()
		{
			const __TimerAccessorType::CountType elapsedTime = endTime - startTime;

			if constexpr (__Units == TimeUnits::nanoseconds)
				timerAccessor.PrintForNsCount(elapsedTime);
			else if constexpr (__Units == TimeUnits::microseconds)
				timerAccessor.PrintForUsCount(elapsedTime);
			else if constexpr (__Units == TimeUnits::milliseconds)
				timerAccessor.PrintForMsCount(elapsedTime);
			else if constexpr (__Units == TimeUnits::seconds)
				timerAccessor.PrintForSCount(elapsedTime);
		}

	public:
		inline MeasuringElapsedTime() noexcept  { setStartTine();          }
		inline ~MeasuringElapsedTime() noexcept { setEndTine();   Print(); }
	};
}