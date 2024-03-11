[![Forks][forks-shield]][forks-url]
[![Stargazers][stars-shield]][stars-url]
[![Issues][issues-shield]][issues-url]
[![MIT License][license-shield]][license-url]

<br />
<div align="center">
  <a href="https://github.com/dan-online/oxidized">
    <img src="assets/oxidized.png" alt="Logo" width="80" height="80">
  </a>

<h3 align="center">oxidized</h3>

  <p align="center">
    self-hosted torrent meta-search application<br /> with a self-building, self-updating database<br /> powered by Rust + PostgreSQL
    <!-- <br /> -->
    <!-- <a href="https://github.com/dan-online/oxidized"><strong>Explore the docs »</strong></a> -->
    <br />
    <br />
    <a href="https://github.com/dan-online/oxidized/issues">Report Bug</a>
    ·
    <a href="https://github.com/dan-online/oxidized/issues">Request Feature</a>
  </p>
</div>

<!-- TABLE OF CONTENTS -->
<details>
  <summary>Table of Contents</summary>
  <ol>
    <li><a href="#about-the-project">About The Project</a></li>
    <li><a href="#getting-started">Getting Started</a></li>
    <li><a href="#license">License</a></li>
    <li><a href="#contact">Contact</a></li>
    <li><a href="#disclaimer">Disclaimer</a></li>
  </ol>
</details>

<!-- ABOUT THE PROJECT -->
## About The Project

Oxidized is a torrent meta-search application designed to simplify and enhance your torrent searching experience. Built with Rust + [Rocket](https://rocket.rs), oxidized delivers a reliable and responsive API for seamless integration. Utilizing PostgreSQL for data management, Oxidized ensures efficient storage and retrieval of torrent information.

### Features

- Tracker integration
    - Queries the top trackers for the most relevant and up-to-date seeder/leecher information
- Self-building, self-updating database ([Spider](https://github.com/boramalper/magnetico/))
    - Watches the DHT (Distributed Hash Table) for new torrents and automatically adds them to the database
    - 7680 indexed torrents/hr leads to 180k+ torrents/day
- Torznab API
    - Allows for integration with popular torrent clients such as Sonarr, Radarr, and Lidarr

Oxidized aims to provide a straightforward and efficient solution for anyone to index torrents, without unnecessary frills or complexities.

## Getting Started

### Docker

The easiest way to get started with Oxidized is to use the provided Docker image

> A sample [docker-compose.yml](docker/docker-compose.yml) is also provided

```bash
# create a network
$ docker network create oxidized

$ docker run -d --net oxidized --name postgresql -e POSTGRES_PASSWORD=oxidized -e POSTGRES_DB=oxidized postgres:alpine

$ docker run -d --net oxidized -e OXIDIZED_DATABASE_URL=postgres://postgres:oxidized@postgresql/oxidized --name oxidized danonline/oxidized
```

### Configuration

Oxidized requires a configuration file to run. By default, it looks for `config.toml` in the current working directory. You can override the [default values](default.toml) using the config.toml or by setting environment variables in the format of: ``OXIDIZED_{SECTION}_{KEY}``. 

For example: `OXIDIZED_DATABASE_URL`

<!-- LICENSE -->
## License

Distributed under the MIT License. See [`LICENSE`](LICENSE) for more information.

<!-- CONTACT -->
## Contact

DanCodes - <dan@dancodes.online>

Project Link: [https://github.com/dan-online/oxidized](https://github.com/dan-online/oxidized)

## Disclaimer

The information and software contained herein are provided solely for educational purposes. I am not responsible for any misuse of this software.

You, the user, are solely responsible for any actions taken or decisions made based on the information or software provided. I am not liable for any direct, indirect, incidental, consequential, or punitive damages arising from the use or misuse of this information or software.

<!-- MARKDOWN LINKS & IMAGES -->
<!-- https://www.markdownguide.org/basic-syntax/#reference-style-links -->
[contributors-shield]: https://img.shields.io/github/contributors/dan-online/oxidized.svg?style=for-the-badge
[contributors-url]: https://github.com/dan-online/oxidized/graphs/contributors
[forks-shield]: https://img.shields.io/github/forks/dan-online/oxidized.svg?style=for-the-badge
[forks-url]: https://github.com/dan-online/oxidized/network/members
[stars-shield]: https://img.shields.io/github/stars/dan-online/oxidized.svg?style=for-the-badge
[stars-url]: https://github.com/dan-online/oxidized/stargazers
[issues-shield]: https://img.shields.io/github/issues/dan-online/oxidized.svg?style=for-the-badge
[issues-url]: https://github.com/dan-online/oxidized/issues
[license-shield]: https://img.shields.io/github/license/dan-online/oxidized.svg?style=for-the-badge
[license-url]: https://github.com/dan-online/oxidized/blob/master/LICENSE.txt

