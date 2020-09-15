FROM mcr.microsoft.com/dotnet/core/aspnet:3.1-alpine AS base
RUN apk add git
WORKDIR /app
EXPOSE 80
EXPOSE 443

FROM mcr.microsoft.com/dotnet/core/sdk:3.1-alpine AS build
WORKDIR /src
COPY 3rd_party /src/3rd_party
COPY coordinator/Karenia.Rurikawa.Coordinator.csproj coordinator/
RUN ls
RUN ls 3rd_party/SplitStream
WORKDIR /src/coordinator/
RUN dotnet restore
COPY coordinator/* /src/coordinator/
RUN dotnet build -c Release -o /app/build -v m

FROM build AS publish
RUN dotnet publish -c Release -o /app/publish

FROM base AS final
WORKDIR /app
COPY --from=publish /app/publish .
ENTRYPOINT ["dotnet", "Karenia.Rurikawa.Coordinator.dll"]
